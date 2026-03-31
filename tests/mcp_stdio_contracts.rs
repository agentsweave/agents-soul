use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

fn test_workspace(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("agents-soul-mcp-stdio-{label}-{suffix}"))
}

#[test]
fn mcp_stdio_mode_serves_real_initialize_tools_list_tool_calls_and_fail_closed_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = test_workspace("subprocess");
    let result =
        (|| {
            std::fs::create_dir_all(workspace.join(".soul"))?;
            write_soul_config(&workspace, "agent.alpha", "Alpha")?;

            let identity_path = workspace.join("identity.json");
            std::fs::write(
                &identity_path,
                include_str!("fixtures/compose_modes/identity_healthy.json"),
            )?;

            let verification_path = workspace.join("verification.json");
            std::fs::write(
                &verification_path,
                include_str!("fixtures/compose_modes/verification_active.json"),
            )?;

            let mut child = spawn_mcp_child()?;
            let mut stdin = child.stdin.take().ok_or("missing child stdin")?;
            let stdout = child.stdout.take().ok_or("missing child stdout")?;
            let mut stdout = BufReader::new(stdout);

            write_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-03-26",
                        "clientInfo": {
                            "name": "agents-soul-contracts",
                            "version": "1.0.0"
                        },
                        "capabilities": {}
                    }
                }),
            )?;
            let initialize = read_message(&mut stdout)?;
            assert_eq!(
                initialize
                    .get("result")
                    .and_then(|result| result.get("protocolVersion"))
                    .and_then(Value::as_str),
                Some("2025-03-26")
            );

            write_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized",
                    "params": {}
                }),
            )?;

            write_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/list",
                    "params": {}
                }),
            )?;
            let tools_list = read_message(&mut stdout)?;
            let tools = tools_list
                .get("result")
                .and_then(|result| result.get("tools"))
                .and_then(Value::as_array)
                .ok_or("tools/list missing tools array")?;
            assert!(tools
            .iter()
            .any(|tool| tool.get("name").and_then(Value::as_str) == Some("compose_context")));
            assert!(
                tools
                    .iter()
                    .any(|tool| tool.get("name").and_then(Value::as_str) == Some("update_traits"))
            );

            write_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": {
                        "name": "compose_context",
                        "arguments": {
                            "workspace_id": workspace.display().to_string(),
                            "agent_id": "agent.alpha",
                            "session_id": "session.alpha",
                            "identity_snapshot_path": identity_path.display().to_string(),
                            "registry_verification_path": verification_path.display().to_string()
                        }
                    }
                }),
            )?;
            let compose_call = read_message(&mut stdout)?;
            let payload_text = content_text(&compose_call)?;
            let payload = serde_json::from_str::<Value>(payload_text)?;
            assert_eq!(
                payload.get("profile_name").and_then(Value::as_str),
                Some("Alpha")
            );
            assert_eq!(
                payload
                    .get("status_summary")
                    .and_then(|summary| summary.get("compose_mode"))
                    .and_then(Value::as_str),
                Some("normal")
            );

            write_message(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": 4,
                    "method": "tools/call",
                    "params": {
                        "name": "compose_context",
                        "arguments": {
                            "workspace_id": workspace.display().to_string(),
                            "agent_id": "",
                            "session_id": "session.revoked",
                            "identity_snapshot_path": identity_path.display().to_string(),
                            "registry_verification_path": verification_path.display().to_string()
                        }
                    }
                }),
            )?;
            let invalid_call = read_message(&mut stdout)?;
            let error = invalid_call.get("error").ok_or("expected error response")?;
            assert_eq!(error.get("code").and_then(Value::as_i64), Some(1001));
            assert_eq!(
                error
                    .get("data")
                    .and_then(|data| data.get("code"))
                    .and_then(Value::as_str),
                Some("soul/request-validation")
            );
            assert_eq!(
                error
                    .get("data")
                    .and_then(|data| data.get("compose_mode_hint")),
                Some(&Value::Null)
            );

            drop(stdin);
            let status = child.wait()?;
            assert!(status.success());

            Ok::<(), Box<dyn std::error::Error>>(())
        })();

    cleanup_workspace(&workspace)?;
    result
}

#[test]
fn mcp_stdio_surfaces_overlay_policy_and_session_only_throttle_effects()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = test_workspace("overlay-throttle");
    let result =
        (|| {
            std::fs::create_dir_all(workspace.join(".soul"))?;
            std::fs::create_dir_all(workspace.join("soul.d"))?;
            write_soul_config(&workspace, "agent.alpha", "Alpha")?;
            std::fs::write(
                workspace.join("soul.d/10-adaptation.toml"),
                r#"
[adaptation]
min_interactions_for_adapt = 1
min_persist_interval_seconds = 900
"#,
            )?;

            let mut child = spawn_mcp_child()?;
            let mut stdin = child.stdin.take().ok_or("missing child stdin")?;
            let stdout = child.stdout.take().ok_or("missing child stdout")?;
            let mut stdout = BufReader::new(stdout);

            initialize_session(&mut stdin, &mut stdout)?;

            let explain = call_tool(
                &mut stdin,
                &mut stdout,
                10,
                "explain_report",
                json!({
                    "workspace_id": workspace.display().to_string(),
                    "agent_id": "agent.alpha",
                    "session_id": "session.alpha"
                }),
            )?;
            assert_eq!(
                explain
                    .get("inspect")
                    .and_then(|inspect| inspect.get("adaptation"))
                    .and_then(|adaptation| adaptation.get("min_interactions_for_adapt"))
                    .and_then(Value::as_u64),
                Some(1)
            );
            assert_eq!(
                explain
                    .get("inspect")
                    .and_then(|inspect| inspect.get("adaptation"))
                    .and_then(|adaptation| adaptation.get("min_persist_interval_seconds"))
                    .and_then(Value::as_u64),
                Some(900)
            );

            let first = call_tool(
                &mut stdin,
                &mut stdout,
                11,
                "record_interaction",
                json!({
                    "workspace_root": workspace.display().to_string(),
                    "event_id": "evt-1",
                    "event": {
                        "agent_id": "agent.alpha",
                        "session_id": "session.alpha",
                        "interaction_type": "review",
                        "outcome": "negative",
                        "signals": [
                            {
                                "kind": "trait",
                                "trait_name": "verbosity",
                                "direction": "decrease",
                                "strength": 1.0,
                                "reason": "user preferred concise responses"
                            }
                        ],
                        "recorded_at": "2026-03-29T02:00:00Z"
                    },
                    "context_json": "{\"surface\":\"mcp\"}",
                    "persist": true
                }),
            )?;
            assert_eq!(
                first.get("effect").and_then(Value::as_str),
                Some("Inserted")
            );

            let second = call_tool(
                &mut stdin,
                &mut stdout,
                12,
                "record_interaction",
                json!({
                    "workspace_root": workspace.display().to_string(),
                    "event_id": "evt-2",
                    "event": {
                        "agent_id": "agent.alpha",
                        "session_id": "session.alpha",
                        "interaction_type": "handoff",
                        "outcome": "positive",
                        "signals": [
                            {
                                "kind": "trait",
                                "trait_name": "warmth",
                                "direction": "increase",
                                "strength": 1.0,
                                "reason": "collaboration stayed smooth"
                            }
                        ],
                        "recorded_at": "2026-03-29T02:05:00Z"
                    },
                    "context_json": "{\"surface\":\"mcp\"}",
                    "persist": true
                }),
            )?;
            assert_eq!(
                second.get("effect").and_then(Value::as_str),
                Some("SessionOnly")
            );
            let candidate = second
                .get("stored_state")
                .and_then(|state| state.get("adaptation_state"))
                .ok_or("missing session-only candidate adaptation state")?;
            assert!(
                candidate
                    .get("trait_overrides")
                    .and_then(|overrides| overrides.get("warmth"))
                    .and_then(Value::as_f64)
                    .unwrap_or_default()
                    > 0.0
            );

            let post_throttle = call_tool(
                &mut stdin,
                &mut stdout,
                13,
                "explain_report",
                json!({
                    "workspace_id": workspace.display().to_string(),
                    "agent_id": "agent.alpha",
                    "session_id": "session.alpha"
                }),
            )?;
            let durable_overrides = post_throttle
                .get("inspect")
                .and_then(|inspect| inspect.get("adaptation"))
                .and_then(|adaptation| adaptation.get("trait_overrides"))
                .and_then(Value::as_array)
                .ok_or("missing inspect adaptation trait overrides")?;
            assert!(durable_overrides.iter().any(|entry| {
                entry.get("trait_name").and_then(Value::as_str) == Some("verbosity")
            }));
            assert!(!durable_overrides.iter().any(|entry| {
                entry.get("trait_name").and_then(Value::as_str) == Some("warmth")
            }));

            drop(stdin);
            let status = child.wait()?;
            assert!(status.success());

            Ok::<(), Box<dyn std::error::Error>>(())
        })();

    cleanup_workspace(&workspace)?;
    result
}

fn spawn_mcp_child() -> Result<Child, Box<dyn std::error::Error>> {
    let binary = std::env::current_exe()?
        .parent()
        .and_then(|path| path.parent())
        .ok_or("failed to resolve target directory from current test binary")?
        .join("agents-soul");
    Ok(Command::new(binary)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?)
}

fn write_message(
    stdin: &mut ChildStdin,
    message: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::to_vec(message)?;
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len())?;
    stdin.write_all(&body)?;
    stdin.flush()?;
    Ok(())
}

fn read_message(stdout: &mut BufReader<ChildStdout>) -> Result<Value, Box<dyn std::error::Error>> {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let read = stdout.read_line(&mut line)?;
        if read == 0 {
            return Err("unexpected EOF while waiting for MCP response".into());
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(value) = trimmed
            .split_once(':')
            .and_then(|(name, value)| name.eq_ignore_ascii_case("content-length").then_some(value))
        {
            content_length = Some(value.trim().parse::<usize>()?);
        }
    }

    let content_length = content_length.ok_or("missing Content-Length in MCP response")?;
    let mut body = vec![0_u8; content_length];
    stdout.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn content_text(message: &Value) -> Result<&str, Box<dyn std::error::Error>> {
    message
        .get("result")
        .and_then(|result| result.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|entry| entry.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| "tools/call missing text payload".into())
}

fn initialize_session(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
) -> Result<(), Box<dyn std::error::Error>> {
    write_message(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "clientInfo": {
                    "name": "agents-soul-contracts",
                    "version": "1.0.0"
                },
                "capabilities": {}
            }
        }),
    )?;
    let initialize = read_message(stdout)?;
    assert_eq!(
        initialize
            .get("result")
            .and_then(|result| result.get("protocolVersion"))
            .and_then(Value::as_str),
        Some("2025-03-26")
    );

    write_message(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    )?;

    Ok(())
}

fn call_tool(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    id: i64,
    name: &str,
    arguments: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    write_message(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        }),
    )?;
    let response = read_message(stdout)?;
    if let Some(error) = response.get("error") {
        return Err(format!("unexpected MCP error for `{name}`: {error}").into());
    }
    let payload_text = content_text(&response)?;
    Ok(serde_json::from_str(payload_text)?)
}

fn cleanup_workspace(workspace: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if workspace.exists() {
        std::fs::remove_dir_all(workspace)?;
    }
    Ok(())
}

fn write_soul_config(
    workspace: &Path,
    agent_id: &str,
    profile_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = agents_soul::domain::SoulConfig {
        agent_id: agent_id.to_owned(),
        profile_name: profile_name.to_owned(),
        ..agents_soul::domain::SoulConfig::default()
    };
    std::fs::write(workspace.join("soul.toml"), toml::to_string(&config)?)?;
    Ok(())
}
