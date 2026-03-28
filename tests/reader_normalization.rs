use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use agents_soul::domain::{
    BehaviorInputs, BehaviorWarning, ComposeMode, ComposeRequest, InputProvenance, InputSourceKind,
    RecoveryState, RegistryStatus, RelationshipMarker, SessionIdentitySnapshot, SoulConfig,
    SourceConfig, VerificationResult, WarningSeverity,
};
use agents_soul::sources::{
    cache::read_cached_inputs_path, identity::IdentityReader, normalize::normalize_inputs,
    registry::RegistryReader,
};
use chrono::Utc;

#[test]
fn normalize_inputs_sorts_and_shapes_compose_inputs() {
    let request = ComposeRequest::new("alpha", "session-1");
    let config = SoulConfig {
        agent_id: "alpha".into(),
        profile_name: "Alpha Builder".into(),
        ..SoulConfig::default()
    };

    let normalized = normalize_inputs(
        &request,
        BehaviorInputs {
            soul_config: config,
            identity_snapshot: Some(SessionIdentitySnapshot {
                agent_id: "alpha".into(),
                display_name: Some("Alpha".into()),
                recovery_state: RecoveryState::Healthy,
                active_commitments: vec!["b".into(), "a".into(), "a".into()],
                durable_preferences: vec!["short".into(), "short".into()],
                relationship_markers: vec![
                    RelationshipMarker {
                        subject: "repo".into(),
                        marker: "owner".into(),
                        note: Some("trusted".into()),
                    },
                    RelationshipMarker {
                        subject: "repo".into(),
                        marker: "owner".into(),
                        note: Some("trusted".into()),
                    },
                ],
                facts: vec!["x".into(), "x".into()],
                warnings: vec![],
                fingerprint: Some("fp".into()),
            }),
            identity_provenance: InputProvenance::live("identity.json"),
            verification_result: Some(VerificationResult {
                status: RegistryStatus::Active,
                standing_level: Some("good".into()),
                reason_code: None,
                verified_at: Some(Utc::now()),
            }),
            verification_provenance: InputProvenance::live("verification.json"),
            reputation_summary: None,
            reader_warnings: vec![BehaviorWarning {
                severity: WarningSeverity::Caution,
                code: "identity_cache_bypassed".into(),
                message: "Identity cache was bypassed.".into(),
            }],
            generated_at: Utc::now(),
            ..BehaviorInputs::default()
        },
    )
    .expect("bundle should normalize");

    assert_eq!(normalized.agent_id, "alpha");
    assert_eq!(normalized.profile_name, "Alpha Builder");
    assert_eq!(normalized.compose_mode_hint, Some(ComposeMode::Normal));
    assert_eq!(normalized.identity_provenance.source, InputSourceKind::Live);
    assert_eq!(
        normalized.verification_provenance.source,
        InputSourceKind::Live
    );
    assert_eq!(
        normalized
            .identity_snapshot
            .as_ref()
            .expect("identity snapshot should survive")
            .active_commitments,
        vec!["a".to_owned(), "b".to_owned()]
    );
    assert!(
        normalized
            .reader_warnings
            .iter()
            .any(|warning| warning.code == "identity_cache_bypassed")
    );
}

#[test]
fn identity_reader_prefers_explicit_path_over_live_and_cache() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("identity-priority");
    let identity_workspace = workspace.join("identity");
    fs::create_dir_all(identity_workspace.join(".soul"))?;
    fs::create_dir_all(workspace.join(".soul"))?;

    write_json(
        identity_workspace.join("session_identity_snapshot.json"),
        r#"{
            "agent_id":"alpha",
            "recovery_state":"healthy",
            "active_commitments":["live"]
        }"#,
    )?;
    write_json(
        workspace.join("explicit_identity.json"),
        r#"{
            "agent_id":"alpha",
            "recovery_state":"healthy",
            "active_commitments":["explicit"]
        }"#,
    )?;
    write_json(
        workspace.join(".soul/context_cache.json"),
        r#"{
            "identity_snapshot":{
                "agent_id":"alpha",
                "recovery_state":"healthy",
                "active_commitments":["cache"]
            }
        }"#,
    )?;

    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();
    request.identity_snapshot_path = Some(
        workspace
            .join("explicit_identity.json")
            .display()
            .to_string(),
    );
    let config = SoulConfig {
        agent_id: "alpha".into(),
        profile_name: "Alpha".into(),
        sources: SourceConfig {
            identity_workspace: identity_workspace.display().to_string(),
            ..SoulConfig::default().sources
        },
        ..SoulConfig::default()
    };

    let selection = IdentityReader.load(&request, &config)?;
    let snapshot = selection.value.expect("identity snapshot");
    assert_eq!(snapshot.active_commitments, vec!["explicit".to_owned()]);
    assert_eq!(selection.provenance.source, InputSourceKind::Explicit);

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn registry_reader_uses_cache_when_live_files_are_missing() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("registry-cache");
    fs::create_dir_all(workspace.join(".soul"))?;
    write_json(
        workspace.join(".soul/context_cache.json"),
        r#"{
            "verification_result":{
                "status":"active",
                "standing_level":"good"
            },
            "reputation_summary":{
                "score_total":4.8,
                "context":["cache-hit"]
            }
        }"#,
    )?;

    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();

    let verification = RegistryReader.load_verification(&request)?;
    let reputation = RegistryReader.load_reputation(&request)?;

    assert_eq!(verification.provenance.source, InputSourceKind::Cache);
    assert_eq!(reputation.provenance.source, InputSourceKind::Cache);
    assert_eq!(
        verification
            .value
            .expect("verification")
            .standing_level
            .as_deref(),
        Some("good")
    );
    assert_eq!(
        reputation.value.expect("reputation").context,
        vec!["cache-hit".to_owned()]
    );

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn normalize_inputs_records_identity_agent_mismatch_as_warning() {
    let request = ComposeRequest::new("alpha", "session-1");
    let config = SoulConfig {
        agent_id: "alpha".into(),
        profile_name: "Alpha Builder".into(),
        ..SoulConfig::default()
    };

    let normalized = normalize_inputs(
        &request,
        BehaviorInputs {
            soul_config: config,
            identity_snapshot: Some(SessionIdentitySnapshot {
                agent_id: "beta".into(),
                display_name: Some("Beta".into()),
                recovery_state: RecoveryState::Healthy,
                active_commitments: vec![],
                durable_preferences: vec![],
                relationship_markers: vec![],
                facts: vec![],
                warnings: vec![],
                fingerprint: None,
            }),
            identity_provenance: InputProvenance::live("identity.json"),
            generated_at: Utc::now(),
            ..BehaviorInputs::default()
        },
    )
    .expect("bundle should normalize");

    assert!(normalized.identity_snapshot.is_none());
    assert_eq!(
        normalized.identity_provenance.source,
        InputSourceKind::Unavailable
    );
    assert!(
        normalized
            .reader_warnings
            .iter()
            .any(|warning| warning.code == "identity_agent_mismatch")
    );
}

#[test]
fn invalid_context_cache_returns_warning_instead_of_failing() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("cache-invalid");
    fs::create_dir_all(workspace.join(".soul"))?;
    write_json(workspace.join(".soul/context_cache.json"), "{not-json")?;

    let cached = read_cached_inputs_path(workspace.join(".soul/context_cache.json"))?;
    assert!(cached.cached_inputs.is_none());
    assert!(
        cached
            .warnings
            .iter()
            .any(|warning| warning.code == "context_cache_invalid")
    );

    cleanup_workspace(&workspace)?;
    Ok(())
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

fn write_json(path: impl AsRef<Path>, contents: &str) -> Result<(), Box<dyn Error>> {
    fs::write(path, contents)?;
    Ok(())
}
