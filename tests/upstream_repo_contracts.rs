use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use agents_soul::{
    AppDeps, ComposeRequest,
    domain::RegistryStatus,
    sources::{identity::IdentityReader, registry::RegistryReader},
};

const IDENTIFY_EXPORT_FIXTURE: &str =
    include_str!("../../ai/tests/fixtures/healthy/export-downstream-context.json");
const REGISTRY_AUTHORITY_FIXTURE: &str =
    include_str!("../../ar/tests/fixtures/registry_agent_record.json");
const REGISTRY_VERIFICATION_ENVELOPE_FIXTURE: &str =
    include_str!("../../ar/tests/fixtures/verification_envelope_mismatch.json");
const HEALTHY_SOUL_CONFIG: &str = include_str!("../examples/workspaces/healthy/soul.toml");

#[test]
fn readers_accept_upstream_identify_and_registry_payloads() -> Result<(), Box<dyn Error>> {
    let identity = IdentityReader;
    let signals = identity.parse_signals(IDENTIFY_EXPORT_FIXTURE)?;
    let snapshot = signals.snapshot.expect("identity snapshot should exist");
    assert_eq!(snapshot.agent_id, "agent-alpha");
    assert_eq!(signals.recovery_state, Some(snapshot.recovery_state));
    assert_eq!(
        snapshot.fingerprint.as_deref(),
        Some(
            "4be6a963f71ee21ea4c18746c2c68ac03dd1af5736c3a3749afd179ade4f5393"
        )
    );

    let registry = RegistryReader::default();
    let verification = registry.parse_verification(REGISTRY_VERIFICATION_ENVELOPE_FIXTURE)?;
    assert_eq!(verification.status, RegistryStatus::Suspended);
    assert_eq!(verification.standing_level.as_deref(), Some("watch"));
    assert_eq!(verification.reason_code.as_deref(), Some("suspended"));

    let snapshot = registry.real.parse_snapshot(REGISTRY_AUTHORITY_FIXTURE)?;
    assert_eq!(
        snapshot.standing.as_ref().map(|standing| standing.status),
        Some(RegistryStatus::Suspended)
    );
    assert!(snapshot.reputation.is_some());
    Ok(())
}

#[test]
fn compose_context_accepts_real_upstream_fixture_shapes() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("upstream-repo-contracts");
    fs::create_dir_all(&workspace)?;
    fs::write(
        workspace.join("soul.toml"),
        HEALTHY_SOUL_CONFIG.replace("agent.alpha", "agent-alpha"),
    )?;
    fs::write(workspace.join("agents_identify.json"), IDENTIFY_EXPORT_FIXTURE)?;
    fs::write(workspace.join("agents_registry.json"), REGISTRY_AUTHORITY_FIXTURE)?;

    let mut request = ComposeRequest::new("agent-alpha", "session-upstream");
    request.workspace_id = workspace.display().to_string();
    request.identity_snapshot_path = Some(
        workspace
            .join("agents_identify.json")
            .display()
            .to_string(),
    );

    let context = AppDeps::default().compose_context(request)?;
    assert_eq!(context.status_summary.compose_mode, agents_soul::ComposeMode::Restricted);
    assert_eq!(
        context.status_summary.registry_status,
        Some(RegistryStatus::Suspended)
    );

    cleanup_workspace(&workspace)?;
    Ok(())
}

fn test_workspace(label: &str) -> PathBuf {
    let unique = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    std::env::temp_dir().join(format!("agents-soul-{label}-{unique}"))
}

fn cleanup_workspace(workspace: &Path) -> Result<(), Box<dyn Error>> {
    if workspace.exists() {
        fs::remove_dir_all(workspace)?;
    }
    Ok(())
}
