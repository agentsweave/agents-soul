use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use agents_soul::{
    ComposeRequest, SoulDependencies,
    domain::{PersonalityProfilePatch, SoulConfig, SourceConfig},
    mcp::tools,
};
use chrono::{DateTime, TimeZone, Utc};

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
fn mcp_compose_tool_matches_shared_compose_context() -> Result<(), Box<dyn Error>> {
    with_request(
        "mcp-compose",
        Some("identity_healthy.json"),
        Some("verification_active.json"),
        |deps, request| {
            let expected = deps.compose_context(request.clone())?;
            let actual = tools::compose_context(deps, request)?;

            ensure(
                actual == expected,
                format!("mcp compose diverged from shared compose output: {actual:?}"),
            )
        },
    )
}

#[test]
fn mcp_prefix_tool_matches_shared_prompt_prefix() -> Result<(), Box<dyn Error>> {
    with_request(
        "mcp-prefix",
        Some("identity_healthy.json"),
        Some("verification_suspended.json"),
        |deps, request| {
            let expected = deps.compose_context(request.clone())?;
            let actual = tools::get_prefix(deps, request)?;

            ensure(
                actual.system_prompt_prefix == expected.system_prompt_prefix,
                format!(
                    "mcp prefix diverged: expected {:?}, got {:?}",
                    expected.system_prompt_prefix, actual.system_prompt_prefix
                ),
            )
        },
    )
}

#[test]
fn mcp_explain_tool_matches_shared_explain_report() -> Result<(), Box<dyn Error>> {
    with_request(
        "mcp-explain",
        Some("identity_degraded.json"),
        Some("verification_active.json"),
        |deps, request| {
            let expected = deps.explain_report(request.clone())?;
            let actual = tools::explain_report(deps, request)?;

            ensure(
                actual == expected,
                "mcp explain diverged from shared explain report".to_owned(),
            )
        },
    )
}

#[test]
fn mcp_traits_tool_matches_shared_trait_projection() -> Result<(), Box<dyn Error>> {
    with_request(
        "mcp-traits",
        Some("identity_healthy.json"),
        Some("verification_active.json"),
        |deps, request| {
            let expected = deps.inspect_report(request.clone())?.traits_only();
            let actual = tools::get_traits(deps, request)?;

            ensure(
                actual == expected,
                "mcp traits projection diverged from shared inspect output".to_owned(),
            )
        },
    )
}

#[test]
fn mcp_heuristics_tool_matches_shared_heuristic_projection() -> Result<(), Box<dyn Error>> {
    with_request(
        "mcp-heuristics",
        Some("identity_healthy.json"),
        Some("verification_active.json"),
        |deps, request| {
            let expected = deps.inspect_report(request.clone())?.heuristics_only();
            let actual = tools::get_heuristics(deps, request)?;

            ensure(
                actual == expected,
                "mcp heuristics projection diverged from shared inspect output".to_owned(),
            )
        },
    )
}

#[test]
fn mcp_update_traits_tool_updates_workspace_config() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("mcp-update-traits");
    let result = (|| {
        fs::create_dir_all(workspace.join("identity-live"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        let patch = PersonalityProfilePatch {
            verbosity: Some(0.83),
            warmth: Some(0.51),
            ..PersonalityProfilePatch::default()
        };
        let deps = SoulDependencies::default();
        let updated = tools::update_traits(&deps, &workspace, patch)?;
        let reloaded = deps.load_soul_config(&workspace.display().to_string())?;

        ensure(
            (updated.trait_baseline.verbosity - 0.83).abs() <= f32::EPSILON,
            format!(
                "updated config verbosity mismatch: {:?}",
                updated.trait_baseline.verbosity
            ),
        )?;
        ensure(
            (updated.trait_baseline.warmth - 0.51).abs() <= f32::EPSILON,
            format!(
                "updated config warmth mismatch: {:?}",
                updated.trait_baseline.warmth
            ),
        )?;
        ensure(
            updated == reloaded,
            "trait update did not persist to workspace config".to_owned(),
        )
    })();

    cleanup_workspace(&workspace)?;
    result
}

fn with_request<F>(
    label: &str,
    identity_fixture: Option<&str>,
    verification_fixture: Option<&str>,
    run: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&SoulDependencies, ComposeRequest) -> Result<(), Box<dyn Error>>,
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
        "verification_suspended.json" => Ok(include_str!(
            "fixtures/compose_modes/verification_suspended.json"
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
    let config = SoulConfig {
        agent_id: agent_id.to_owned(),
        profile_name: profile_name.to_owned(),
        sources: SourceConfig {
            identity_workspace: workspace.join("identity-live").display().to_string(),
            ..SoulConfig::default().sources
        },
        ..SoulConfig::default()
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
