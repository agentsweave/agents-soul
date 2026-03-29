use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use agents_soul::{
    ComposeMode, SoulDependencies, SoulError, api, app, cli, domain::ComposeRequest, mcp,
};
use chrono::{DateTime, TimeZone, Utc};

#[derive(Debug, Clone)]
struct FixedClock;

impl agents_soul::app::deps::ComposeClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 29, 8, 0, 0)
            .single()
            .expect("fixed timestamp should be valid")
    }
}

#[test]
fn crate_layout_matches_planned_boundary_order() -> Result<(), String> {
    let layout = app::runtime::crate_layout()
        .iter()
        .map(|layer| layer.name())
        .collect::<Vec<_>>();

    let expected_layout = vec![
        "app",
        "domain",
        "sources",
        "services",
        "adaptation",
        "storage",
        "cli",
        "api",
        "mcp",
    ];

    if layout != expected_layout {
        return Err(format!(
            "unexpected crate layout: expected {expected_layout:?}, got {layout:?}"
        ));
    }

    let transport_layers = app::runtime::transport_layers()
        .iter()
        .map(|layer| layer.name())
        .collect::<Vec<_>>();
    let expected_transport = vec!["cli", "api", "mcp"];

    if transport_layers != expected_transport {
        return Err(format!(
            "unexpected transport layers: expected {expected_transport:?}, got {transport_layers:?}"
        ));
    }

    Ok(())
}

#[test]
fn api_and_mcp_delegate_to_the_shared_compose_service() -> Result<(), String> {
    let workspace = test_workspace("boundary-parity");
    fs::create_dir_all(&workspace).map_err(|error| error.to_string())?;
    write_soul_config(&workspace, "agent.alpha", "Alpha").map_err(|error| error.to_string())?;

    let deps = SoulDependencies::default().with_clock(FixedClock);
    let mut request = ComposeRequest::new("agent.alpha", "session.alpha");
    request.workspace_id = workspace.display().to_string();
    let expected = deps
        .compose_context(request.clone())
        .map_err(|error| error.to_string())?;
    let api_result =
        api::compose::compose_context(&deps, request.clone()).map_err(|error| error.to_string())?;
    let mcp_result =
        mcp::tools::compose_context(&deps, request).map_err(|error| error.to_string())?;

    if api_result != expected {
        return Err(format!(
            "api compose diverged from shared service: expected {expected:?}, got {api_result:?}"
        ));
    }

    if mcp_result != expected {
        return Err(format!(
            "mcp compose diverged from shared service: expected {expected:?}, got {mcp_result:?}"
        ));
    }

    cleanup_workspace(&workspace).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn transports_share_the_same_core_error_mapping() {
    let deps = SoulDependencies::default();
    let degraded = SoulError::RegistryUnavailable;
    let degraded_expected = app::errors::map_soul_error(&degraded);

    assert_eq!(deps.map_error(&degraded), degraded_expected);
    assert_eq!(
        cli::compose::map_compose_error(&degraded),
        degraded_expected
    );
    assert_eq!(
        api::compose::map_compose_error(&degraded),
        degraded_expected
    );
    assert_eq!(mcp::tools::map_compose_error(&degraded), degraded_expected);
    assert_eq!(
        degraded_expected.compose_mode_hint,
        Some(ComposeMode::Degraded)
    );

    let fail_closed = SoulError::RevokedStanding;
    let fail_closed_expected = app::errors::map_soul_error(&fail_closed);

    assert_eq!(
        fail_closed_expected.compose_mode_hint,
        Some(ComposeMode::FailClosed)
    );
    assert_eq!(
        cli::compose::map_compose_error(&fail_closed),
        fail_closed_expected
    );
}

fn test_workspace(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
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
        ..agents_soul::domain::SoulConfig::default()
    };
    fs::write(workspace.join("soul.toml"), toml::to_string(&config)?)?;
    Ok(())
}
