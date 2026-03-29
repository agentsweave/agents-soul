use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use agents_soul::{
    BehavioralContext, ComposeMode, ComposeRequest, SoulDependencies,
    domain::{RegistryStatus, SoulConfig, SourceConfig},
};

#[test]
fn compose_restricted_mode_surfaces_suspension_warning_and_status() -> Result<(), Box<dyn Error>> {
    with_composed_context(
        "restricted-mode",
        Some("identity_healthy.json"),
        Some("verification_suspended.json"),
        |context| {
            ensure(
                context.status_summary.compose_mode == ComposeMode::Restricted,
                format!(
                    "expected restricted compose mode, got {:?}",
                    context.status_summary.compose_mode
                ),
            )?;
            ensure(
                context.status_summary.identity_loaded,
                "expected identity_loaded to be true".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_verified,
                "expected registry_verified to be true".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_status == Some(RegistryStatus::Suspended),
                format!(
                    "expected suspended registry status, got {:?}",
                    context.status_summary.registry_status
                ),
            )?;

            let codes = warning_codes(&context);
            ensure(
                codes.contains(&"registry_suspended"),
                format!("expected registry_suspended warning, got {codes:?}"),
            )?;
            ensure(
                codes.contains(&"compose_restricted"),
                format!("expected compose_restricted warning, got {codes:?}"),
            )?;
            ensure(
                context
                    .system_prompt_prefix
                    .starts_with("RESTRICTED: identity suspended."),
                format!("unexpected prefix: {}", context.system_prompt_prefix),
            )?;
            Ok(())
        },
    )
}

#[test]
fn compose_fail_closed_mode_drops_loaded_context_and_escalates() -> Result<(), Box<dyn Error>> {
    with_composed_context(
        "fail-closed-mode",
        Some("identity_healthy.json"),
        Some("verification_revoked.json"),
        |context| {
            ensure(
                context.status_summary.compose_mode == ComposeMode::FailClosed,
                format!(
                    "expected fail-closed compose mode, got {:?}",
                    context.status_summary.compose_mode
                ),
            )?;
            ensure(
                context.status_summary.identity_loaded,
                "expected identity_loaded to be true".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_verified,
                "expected registry_verified to be true".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_status == Some(RegistryStatus::Revoked),
                format!(
                    "expected revoked registry status, got {:?}",
                    context.status_summary.registry_status
                ),
            )?;

            let codes = warning_codes(&context);
            ensure(
                codes.contains(&"registry_revoked"),
                format!("expected registry_revoked warning, got {codes:?}"),
            )?;
            ensure(
                codes.contains(&"compose_fail_closed"),
                format!("expected compose_fail_closed warning, got {codes:?}"),
            )?;
            ensure(
                context.active_commitments.is_empty(),
                "expected fail-closed commitments to be stripped".to_owned(),
            )?;
            ensure(
                context.relationship_context.is_empty(),
                "expected fail-closed relationships to be stripped".to_owned(),
            )?;
            ensure(
                context.adaptive_notes.is_empty(),
                "expected fail-closed adaptive notes to be stripped".to_owned(),
            )?;
            ensure(
                context
                    .system_prompt_prefix
                    .starts_with("FAIL-CLOSED: identity revoked."),
                format!("unexpected prefix: {}", context.system_prompt_prefix),
            )?;
            Ok(())
        },
    )
}

#[test]
fn compose_degraded_mode_marks_reduced_autonomy() -> Result<(), Box<dyn Error>> {
    with_composed_context(
        "degraded-mode",
        Some("identity_degraded.json"),
        Some("verification_active.json"),
        |context| {
            ensure(
                context.status_summary.compose_mode == ComposeMode::Degraded,
                format!(
                    "expected degraded compose mode, got {:?}",
                    context.status_summary.compose_mode
                ),
            )?;
            ensure(
                context.status_summary.identity_loaded,
                "expected identity_loaded to be true".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_verified,
                "expected registry_verified to be true".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_status == Some(RegistryStatus::Active),
                format!(
                    "expected active registry status, got {:?}",
                    context.status_summary.registry_status
                ),
            )?;

            let codes = warning_codes(&context);
            ensure(
                codes == vec!["compose_degraded", "identity_degraded", "reputation_unavailable"],
                format!("unexpected degraded warning codes: {codes:?}"),
            )?;
            ensure(
                context.trait_profile.initiative < context.baseline_trait_profile.initiative,
                "degraded mode should reduce initiative".to_owned(),
            )?;
            Ok(())
        },
    )
}

#[test]
fn compose_baseline_only_mode_preserves_verified_status_without_identity()
-> Result<(), Box<dyn Error>> {
    with_composed_context(
        "baseline-only-mode",
        None,
        Some("verification_active.json"),
        |context| {
            ensure(
                context.status_summary.compose_mode == ComposeMode::BaselineOnly,
                format!(
                    "expected baseline-only compose mode, got {:?}",
                    context.status_summary.compose_mode
                ),
            )?;
            ensure(
                !context.status_summary.identity_loaded,
                "expected identity_loaded to be false".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_verified,
                "expected registry_verified to be true".to_owned(),
            )?;
            ensure(
                context.status_summary.registry_status == Some(RegistryStatus::Active),
                format!(
                    "expected active registry status, got {:?}",
                    context.status_summary.registry_status
                ),
            )?;

            let codes = warning_codes(&context);
            ensure(
                codes == vec!["baseline_only", "identity_unavailable", "reputation_unavailable"],
                format!("unexpected baseline-only warning codes: {codes:?}"),
            )?;
            ensure(
                context.active_commitments.is_empty(),
                "expected baseline-only commitments to stay empty".to_owned(),
            )?;
            ensure(
                context.relationship_context.is_empty(),
                "expected baseline-only relationships to stay empty".to_owned(),
            )?;
            Ok(())
        },
    )
}

fn with_composed_context<F>(
    label: &str,
    identity_fixture: Option<&str>,
    verification_fixture: Option<&str>,
    assert_context: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(BehavioralContext) -> Result<(), Box<dyn Error>>,
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

        let context = SoulDependencies::default().compose_context(request)?;
        assert_context(context)
    })();

    cleanup_workspace(&workspace)?;
    result
}

fn warning_codes(context: &BehavioralContext) -> Vec<&str> {
    context
        .warnings
        .iter()
        .map(|warning| warning.code.as_str())
        .collect::<Vec<_>>()
}

fn fixture(name: &str) -> Result<&'static str, Box<dyn Error>> {
    match name {
        "identity_healthy.json" => Ok(include_str!("fixtures/compose_modes/identity_healthy.json")),
        "identity_degraded.json" => {
            Ok(include_str!("fixtures/compose_modes/identity_degraded.json"))
        }
        "verification_active.json" => {
            Ok(include_str!("fixtures/compose_modes/verification_active.json"))
        }
        "verification_suspended.json" => Ok(include_str!(
            "fixtures/compose_modes/verification_suspended.json"
        )),
        "verification_revoked.json" => {
            Ok(include_str!("fixtures/compose_modes/verification_revoked.json"))
        }
        other => Err(format!("unknown compose-mode fixture `{other}`").into()),
    }
}

fn test_workspace(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}"))
}

fn ensure(condition: bool, message: String) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
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
