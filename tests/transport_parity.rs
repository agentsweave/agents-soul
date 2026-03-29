use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use agents_soul::{
    COMPOSE_ROUTE, ComposeRequest, EXPLAIN_ROUTE, HttpRequest, SoulDependencies, SoulError, api,
    cli, handle_request, map_soul_error, mcp,
};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

#[derive(Debug, Clone)]
struct FixedClock;

impl agents_soul::app::deps::ComposeClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 29, 8, 0, 0)
            .single()
            .unwrap_or_else(Utc::now)
    }
}

#[test]
fn compose_payloads_match_across_cli_rest_and_mcp() -> Result<(), Box<dyn Error>> {
    let cli_projection = with_request(
        "transport-compose-parity-cli",
        Some("identity_healthy.json"),
        Some("verification_active.json"),
        |deps, request| {
            let cli = cli::compose::compose_cmd(deps, compose_cmd(&request))?;
            ensure(
                cli.rendered.is_none(),
                "cli compose JSON mode should not render text output".to_owned(),
            )?;
            let cli_value = serde_json::to_value(&cli.output)?;
            Ok(compose_projection(&cli_value))
        },
    )?;
    let rest_projection = with_request(
        "transport-compose-parity-rest",
        Some("identity_healthy.json"),
        Some("verification_active.json"),
        |deps, request| {
            let rest_value = rest_read_value(
                COMPOSE_ROUTE.method.as_str(),
                COMPOSE_ROUTE.path,
                &request,
                deps,
            )?;
            Ok(compose_projection(&rest_value))
        },
    )?;
    let mcp_projection = with_request(
        "transport-compose-parity-mcp",
        Some("identity_healthy.json"),
        Some("verification_active.json"),
        |deps, request| {
            let mcp_value =
                serde_json::to_value(mcp::tools::compose_context(deps, request.clone())?)?;
            Ok(compose_projection(&mcp_value))
        },
    )?;
    assert_compose_projection_matches("rest", &rest_projection, &cli_projection)?;
    assert_compose_projection_matches("mcp", &mcp_projection, &cli_projection)
}

#[test]
fn explain_payloads_match_across_cli_rest_and_mcp() -> Result<(), Box<dyn Error>> {
    let cli_projection = with_request(
        "transport-explain-parity-cli",
        Some("identity_degraded.json"),
        Some("verification_active.json"),
        |deps, request| {
            let cli = cli::explain::explain_cmd(deps, explain_cmd(&request))?;
            ensure(
                cli.rendered.is_none(),
                "cli explain JSON mode should not render text output".to_owned(),
            )?;
            let cli_value = serde_json::to_value(&cli.output)?;
            Ok(explain_projection(&cli_value))
        },
    )?;
    let rest_projection = with_request(
        "transport-explain-parity-rest",
        Some("identity_degraded.json"),
        Some("verification_active.json"),
        |deps, request| {
            let rest_value = rest_read_value(
                EXPLAIN_ROUTE.method.as_str(),
                EXPLAIN_ROUTE.path,
                &request,
                deps,
            )?;
            Ok(explain_projection(&rest_value))
        },
    )?;
    let mcp_projection = with_request(
        "transport-explain-parity-mcp",
        Some("identity_degraded.json"),
        Some("verification_active.json"),
        |deps, request| {
            let mcp_value =
                serde_json::to_value(mcp::tools::explain_report(deps, request.clone())?)?;
            Ok(explain_projection(&mcp_value))
        },
    )?;
    assert_explain_projection_matches("rest", &rest_projection, &cli_projection)?;
    assert_explain_projection_matches("mcp", &mcp_projection, &cli_projection)?;
    ensure(
        cli_projection["inspect"]["warnings"] == rest_projection["inspect"]["warnings"],
        "rest explain warnings drifted from CLI explain warnings".to_owned(),
    )
}

#[test]
fn compose_and_explain_errors_share_transport_mapping_for_major_modes() {
    let cases = [
        SoulError::RegistryUnavailable,
        SoulError::RevokedStanding,
        SoulError::InvalidConfig("missing soul.toml".into()),
    ];

    for error in cases {
        let mapped = map_soul_error(&error);

        let cli_compose = cli::compose::map_compose_error(&error);
        let cli_explain = cli::explain::map_explain_error(&error);
        let api_compose = api::compose::compose_error_response(&error);
        let api_explain = api::explain::explain_error_response(&error);
        let mcp_error = mcp::tools::compose_tool_error(&error);

        assert_eq!(cli_compose, mapped);
        assert_eq!(cli_explain, mapped);
        assert_eq!(api_compose.status, mapped.http_status);
        assert_eq!(api_explain.status, mapped.http_status);
        assert_eq!(api_compose.body.error.code, mapped.code);
        assert_eq!(api_explain.body.error.code, mapped.code);
        assert_eq!(api_compose.body.error.category, mapped.category);
        assert_eq!(api_explain.body.error.category, mapped.category);
        assert_eq!(
            api_compose.body.error.compose_mode_hint,
            mapped.compose_mode_hint
        );
        assert_eq!(
            api_explain.body.error.compose_mode_hint,
            mapped.compose_mode_hint
        );
        assert_eq!(mcp_error.code, mapped.mcp_error_name);
        assert_eq!(mcp_error.data.error_code, mapped.mcp_error_code);
        assert_eq!(mcp_error.data.http_status, mapped.http_status);
        assert_eq!(mcp_error.data.cli_exit_code, mapped.cli_exit_code);
    }
}

fn rest_read_value(
    method: &str,
    path: &str,
    request: &ComposeRequest,
    deps: &SoulDependencies,
) -> Result<Value, Box<dyn Error>> {
    let body_json = serde_json::to_string(request)?;
    let response = handle_request(
        deps,
        HttpRequest {
            method,
            path,
            body_json: &body_json,
        },
    );
    ensure(
        response.status == 200,
        format!(
            "expected REST success for {method} {path}, got {}",
            response.status
        ),
    )?;

    let body: Value = serde_json::from_str(&response.body_json)?;
    body.get("data")
        .cloned()
        .ok_or_else(|| format!("REST response for {method} {path} omitted `data` envelope").into())
}

fn compose_cmd(request: &ComposeRequest) -> cli::compose::ComposeCmd {
    cli::compose::ComposeCmd {
        workspace: request.workspace_id.clone(),
        json: true,
        prefix_only: false,
        identity_snapshot_path: request.identity_snapshot_path.clone(),
        registry_verification_path: request.registry_verification_path.clone(),
        registry_reputation_path: request.registry_reputation_path.clone(),
        no_reputation: !request.include_reputation,
        no_relationships: !request.include_relationships,
        no_commitments: !request.include_commitments,
        session_id: request.session_id.clone(),
    }
}

fn explain_cmd(request: &ComposeRequest) -> cli::explain::ExplainCmd {
    cli::explain::ExplainCmd {
        workspace: request.workspace_id.clone(),
        json: true,
        identity_snapshot_path: request.identity_snapshot_path.clone(),
        registry_verification_path: request.registry_verification_path.clone(),
        registry_reputation_path: request.registry_reputation_path.clone(),
        no_reputation: !request.include_reputation,
        no_relationships: !request.include_relationships,
        no_commitments: !request.include_commitments,
        session_id: request.session_id.clone(),
    }
}

fn compose_projection(value: &Value) -> Value {
    normalize_json_numbers(serde_json::json!({
        "agent_id": value["agent_id"].clone(),
        "profile_name": value["profile_name"].clone(),
        "status_summary": value["status_summary"].clone(),
        "baseline_trait_profile": value["baseline_trait_profile"].clone(),
        "trait_profile": value["trait_profile"].clone(),
        "communication_rules": value["communication_rules"].clone(),
        "decision_rules": value["decision_rules"].clone(),
        "active_commitments": value["active_commitments"].clone(),
        "relationship_context": value["relationship_context"].clone(),
        "adaptive_notes": value["adaptive_notes"].clone(),
        "warnings": value["warnings"].clone(),
        "system_prompt_prefix": value["system_prompt_prefix"].clone(),
    }))
}

fn explain_projection(value: &Value) -> Value {
    normalize_json_numbers(serde_json::json!({
        "agent_id": value["agent_id"].clone(),
        "profile_name": value["profile_name"].clone(),
        "status_summary": value["status_summary"].clone(),
        "inspect": {
            "status_summary": value["inspect"]["status_summary"].clone(),
            "traits": value["inspect"]["traits"].clone(),
            "heuristics": value["inspect"]["heuristics"].clone(),
            "adaptation": value["inspect"]["adaptation"].clone(),
            "warnings": value["inspect"]["warnings"].clone(),
        },
    }))
}

fn assert_compose_projection_matches(
    transport: &str,
    actual: &Value,
    expected: &Value,
) -> Result<(), Box<dyn Error>> {
    for field in [
        "agent_id",
        "profile_name",
        "status_summary",
        "baseline_trait_profile",
        "trait_profile",
        "communication_rules",
        "decision_rules",
        "active_commitments",
        "relationship_context",
        "adaptive_notes",
        "warnings",
        "system_prompt_prefix",
    ] {
        ensure(
            actual[field] == expected[field],
            format!(
                "{transport} compose field `{field}` diverged: actual={:?} expected={:?}",
                actual[field], expected[field]
            ),
        )?;
    }

    Ok(())
}

fn assert_explain_projection_matches(
    transport: &str,
    actual: &Value,
    expected: &Value,
) -> Result<(), Box<dyn Error>> {
    for field in ["agent_id", "profile_name", "status_summary"] {
        ensure(
            actual[field] == expected[field],
            format!(
                "{transport} explain field `{field}` diverged: actual={:?} expected={:?}",
                actual[field], expected[field]
            ),
        )?;
    }

    for inspect_field in [
        "status_summary",
        "traits",
        "heuristics",
        "adaptation",
        "warnings",
    ] {
        ensure(
            actual["inspect"][inspect_field] == expected["inspect"][inspect_field],
            format!(
                "{transport} explain inspect field `{inspect_field}` diverged: actual={:?} expected={:?}",
                actual["inspect"][inspect_field], expected["inspect"][inspect_field]
            ),
        )?;
    }

    Ok(())
}

fn normalize_json_numbers(value: Value) -> Value {
    match value {
        Value::Number(number) => number
            .as_f64()
            .and_then(|value| {
                let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
                serde_json::Number::from_f64(rounded)
            })
            .map(Value::Number)
            .unwrap_or(Value::Number(number)),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(normalize_json_numbers)
                .collect::<Vec<_>>(),
        ),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, normalize_json_numbers(value)))
                .collect(),
        ),
        other => other,
    }
}

fn with_request<T, F>(
    label: &str,
    identity_fixture: Option<&str>,
    verification_fixture: Option<&str>,
    run: F,
) -> Result<T, Box<dyn Error>>
where
    F: FnOnce(&SoulDependencies, ComposeRequest) -> Result<T, Box<dyn Error>>,
{
    let workspace = test_workspace(label);
    let result = (|| {
        fs::create_dir_all(workspace.join("identity-live"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        let mut request = ComposeRequest::new("agent.alpha", "session.alpha");
        request.workspace_id = workspace.display().to_string();

        if let Some(identity_fixture) = identity_fixture {
            let identity_path = workspace.join("identity.json");
            fs::write(&identity_path, fixture(identity_fixture)?)?;
            request.identity_snapshot_path = Some(identity_path.display().to_string());
        }

        if let Some(verification_fixture) = verification_fixture {
            let verification_path = workspace.join("verification.json");
            fs::write(&verification_path, fixture(verification_fixture)?)?;
            request.registry_verification_path = Some(verification_path.display().to_string());
        }

        let deps = SoulDependencies::default().with_clock(FixedClock);
        run(&deps, request)
    })();

    cleanup_workspace(&workspace)?;
    result
}

fn fixture(name: &str) -> Result<&'static str, Box<dyn Error>> {
    match name {
        "identity_healthy.json" => Ok(include_str!("fixtures/compose_modes/identity_healthy.json")),
        "identity_degraded.json" => Ok(include_str!(
            "fixtures/compose_modes/identity_degraded.json"
        )),
        "verification_active.json" => Ok(include_str!(
            "fixtures/compose_modes/verification_active.json"
        )),
        other => Err(format!("unknown fixture `{other}`").into()),
    }
}

fn test_workspace(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}"))
}

fn cleanup_workspace(workspace: &Path) -> Result<(), Box<dyn Error>> {
    if workspace.exists() {
        fs::remove_dir_all(workspace)?;
    }
    Ok(())
}

fn write_soul_config(
    workspace: &Path,
    agent_id: &str,
    profile_name: &str,
) -> Result<(), Box<dyn Error>> {
    let config = agents_soul::domain::SoulConfig {
        agent_id: agent_id.to_owned(),
        profile_name: profile_name.to_owned(),
        sources: agents_soul::domain::SourceConfig {
            identity_workspace: workspace.join("identity-live").display().to_string(),
            ..agents_soul::domain::SoulConfig::default().sources
        },
        ..agents_soul::domain::SoulConfig::default()
    };
    fs::write(workspace.join("soul.toml"), toml::to_string(&config)?)?;
    Ok(())
}

fn ensure(condition: bool, message: String) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
