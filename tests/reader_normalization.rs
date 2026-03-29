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
    cache::{
        CachedFreshness, CachedInputs, context_cache_key, read_cached_inputs,
        read_cached_inputs_path, write_cached_inputs,
    },
    identity::IdentityReader,
    normalize::normalize_inputs,
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
    assert_eq!(
        normalized.upstream.identity.provenance.source,
        InputSourceKind::Live
    );
    assert_eq!(
        normalized.upstream.registry.verification_provenance.source,
        InputSourceKind::Live
    );
    assert_eq!(
        normalized
            .upstream
            .identity
            .snapshot
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
            "snapshot": {
                "agent_id":"alpha",
                "recovery_state":"healthy",
                "active_commitments":["explicit"]
            },
            "recovery_state":"healthy"
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
    let provenance = selection.provenance.clone();
    let signals = selection.value.expect("identity signals");
    let snapshot = signals.snapshot.expect("identity snapshot");
    assert_eq!(snapshot.active_commitments, vec!["explicit".to_owned()]);
    assert_eq!(provenance.source, InputSourceKind::Explicit);
    assert_eq!(signals.recovery_state, Some(RecoveryState::Healthy));

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn registry_reader_uses_cache_when_live_files_are_missing() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("registry-cache");
    fs::create_dir_all(workspace.join(".soul"))?;

    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();
    write_cached_inputs(
        &request,
        &CachedInputs {
            cache_key: None,
            freshness: Some(CachedFreshness {
                config_hash: None,
                adaptation_hash: None,
                identity_fingerprint: None,
                registry_verification_at: None,
            }),
            identity_snapshot: None,
            verification_result: Some(VerificationResult {
                status: RegistryStatus::Active,
                standing_level: Some("good".to_owned()),
                reason_code: None,
                verified_at: None,
            }),
            reputation_summary: Some(agents_soul::domain::ReputationSummary {
                score_total: Some(4.8),
                score_recent_30d: None,
                last_event_at: None,
                context: vec!["cache-hit".to_owned()],
            }),
        },
    )?;

    let verification = RegistryReader::default().load_verification(&request)?;
    let reputation = RegistryReader::default().load_reputation(&request)?;

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
fn registry_reader_supports_combined_registry_snapshot_file() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("registry-snapshot");
    fs::create_dir_all(workspace.join(".soul"))?;
    write_json(
        workspace.join("agents_registry.json"),
        r#"{
            "standing": {
                "status": "suspended",
                "standing_level": "watch",
                "reason_code": "manual-review"
            },
            "reputation": {
                "score_total": 2.4,
                "score_recent_30d": 1.5,
                "context": ["recent incident", "manual review"]
            }
        }"#,
    )?;

    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();

    let reader = RegistryReader::default();
    let verification = reader.load_verification(&request)?;
    let reputation = reader.load_reputation(&request)?;
    let snapshot = reader.load_snapshot(&request)?;

    assert_eq!(verification.provenance.source, InputSourceKind::Live);
    assert_eq!(reputation.provenance.source, InputSourceKind::Live);
    assert_eq!(
        verification.value.expect("standing").status,
        RegistryStatus::Suspended
    );
    assert_eq!(reputation.value.expect("reputation").score_total, Some(2.4));

    let snapshot = snapshot.value.expect("snapshot");
    assert_eq!(
        snapshot.standing.expect("standing").reason_code.as_deref(),
        Some("manual-review")
    );
    assert_eq!(
        snapshot.reputation.expect("reputation").context,
        vec!["recent incident".to_owned(), "manual review".to_owned()]
    );

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn identity_reader_supports_health_only_identify_fixture() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("identify-health-only");
    let identity_workspace = workspace.join("identity");
    fs::create_dir_all(identity_workspace.join(".soul"))?;

    write_json(
        identity_workspace.join("agents_identify.json"),
        r#"{
            "recovery_state":"degraded"
        }"#,
    )?;

    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();
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
    let provenance = selection.provenance.clone();
    let signals = selection.value.expect("identify signals");

    assert_eq!(provenance.source, InputSourceKind::Live);
    assert_eq!(signals.recovery_state, Some(RecoveryState::Degraded));
    assert!(signals.snapshot.is_none());

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

    assert!(normalized.upstream.identity.snapshot.is_none());
    assert_eq!(
        normalized.upstream.identity.provenance.source,
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
fn normalize_inputs_caps_offline_fail_closed_to_baseline_only_without_identity() {
    let request = ComposeRequest::new("alpha", "session-1");
    let mut config = SoulConfig {
        agent_id: "alpha".into(),
        profile_name: "Alpha Builder".into(),
        ..SoulConfig::default()
    };
    config.limits.offline_registry_behavior =
        agents_soul::domain::OfflineRegistryBehavior::FailClosed;

    let normalized = normalize_inputs(
        &request,
        BehaviorInputs {
            soul_config: config,
            generated_at: Utc::now(),
            ..BehaviorInputs::default()
        },
    )
    .expect("bundle should normalize");

    assert_eq!(
        normalized.compose_mode_hint,
        Some(ComposeMode::BaselineOnly)
    );
}

#[test]
fn normalize_inputs_preserves_identify_health_without_snapshot() {
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
            identity_recovery_state: Some(RecoveryState::Recovering),
            generated_at: Utc::now(),
            ..BehaviorInputs::default()
        },
    )
    .expect("bundle should normalize");

    assert!(normalized.upstream.identity.snapshot.is_none());
    assert_eq!(
        normalized.upstream.identity.recovery_state,
        Some(RecoveryState::Recovering)
    );
    assert_eq!(normalized.compose_mode_hint, Some(ComposeMode::Degraded));
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

#[test]
fn context_cache_with_mismatched_key_is_bypassed() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("cache-key-mismatch");
    fs::create_dir_all(workspace.join(".soul"))?;
    let mut request = ComposeRequest::new("alpha", "session-1");
    request.workspace_id = workspace.display().to_string();
    write_json(
        workspace.join(".soul/context_cache.json"),
        r#"{
            "cache_key":"ctx_deadbeef",
            "identity_snapshot":{
                "agent_id":"alpha",
                "recovery_state":"healthy",
                "active_commitments":["cache"]
            }
        }"#,
    )?;

    let cached = read_cached_inputs(&request)?;
    assert!(cached.cached_inputs.is_none());
    assert!(
        cached
            .warnings
            .iter()
            .any(|warning| warning.code == "context_cache_key_mismatch")
    );

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn cache_writer_persists_derived_key_for_future_reads() -> Result<(), Box<dyn Error>> {
    let workspace = test_workspace("cache-write-keyed");
    fs::create_dir_all(workspace.join(".soul"))?;
    let mut request = ComposeRequest::new("alpha", "session-cache");
    request.workspace_id = workspace.display().to_string();
    let cached_inputs = CachedInputs {
        cache_key: None,
        freshness: Some(CachedFreshness {
            config_hash: None,
            adaptation_hash: None,
            identity_fingerprint: Some("fp".to_owned()),
            registry_verification_at: None,
        }),
        identity_snapshot: Some(SessionIdentitySnapshot {
            agent_id: "alpha".to_owned(),
            display_name: Some("Alpha".to_owned()),
            recovery_state: RecoveryState::Healthy,
            active_commitments: vec!["cache".to_owned()],
            durable_preferences: Vec::new(),
            relationship_markers: Vec::new(),
            facts: Vec::new(),
            warnings: Vec::new(),
            fingerprint: Some("fp".to_owned()),
        }),
        verification_result: None,
        reputation_summary: None,
    };

    write_cached_inputs(&request, &cached_inputs)?;
    let loaded = read_cached_inputs(&request)?;
    let loaded_inputs = loaded.cached_inputs.expect("cache should load");

    assert!(loaded.warnings.is_empty());
    assert_eq!(
        loaded_inputs.cache_key.as_deref(),
        Some(context_cache_key(&request).as_str())
    );
    assert_eq!(
        loaded_inputs
            .identity_snapshot
            .expect("identity snapshot")
            .agent_id,
        "alpha"
    );

    cleanup_workspace(&workspace)?;
    Ok(())
}

#[test]
fn normalize_inputs_applies_request_gates_and_degraded_reputation_fallbacks() {
    let mut request = ComposeRequest::new("alpha", "session-1");
    request.include_commitments = false;
    request.include_relationships = false;
    request.include_reputation = false;

    let normalized = normalize_inputs(
        &request,
        BehaviorInputs {
            soul_config: SoulConfig {
                agent_id: "alpha".into(),
                profile_name: "Alpha Builder".into(),
                ..SoulConfig::default()
            },
            identity_snapshot: Some(SessionIdentitySnapshot {
                agent_id: "alpha".into(),
                display_name: Some("Alpha".into()),
                recovery_state: RecoveryState::Recovering,
                active_commitments: vec!["keep-me-hidden".into()],
                durable_preferences: vec!["terse".into()],
                relationship_markers: vec![RelationshipMarker {
                    subject: "repo".into(),
                    marker: "owner".into(),
                    note: Some("trusted".into()),
                }],
                facts: vec!["fact".into()],
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
            reputation_summary: Some(agents_soul::domain::ReputationSummary {
                score_total: Some(1.8),
                score_recent_30d: Some(2.2),
                last_event_at: None,
                context: vec!["recent review".into()],
            }),
            reputation_provenance: InputProvenance::live("reputation.json"),
            generated_at: Utc::now(),
            ..BehaviorInputs::default()
        },
    )
    .expect("bundle should normalize");

    let snapshot = normalized
        .upstream
        .identity
        .snapshot
        .as_ref()
        .expect("identity snapshot should survive");

    assert!(snapshot.active_commitments.is_empty());
    assert!(snapshot.relationship_markers.is_empty());
    assert!(normalized.upstream.registry.reputation.is_none());
    assert_eq!(
        normalized.upstream.registry.reputation_provenance.source,
        InputSourceKind::Unavailable
    );
    assert_eq!(normalized.compose_mode_hint, Some(ComposeMode::Degraded));
}

#[test]
fn normalize_inputs_uses_suspended_registry_as_restricted_mode_hint() {
    let request = ComposeRequest::new("alpha", "session-1");

    let normalized = normalize_inputs(
        &request,
        BehaviorInputs {
            soul_config: SoulConfig {
                agent_id: "alpha".into(),
                profile_name: "Alpha Builder".into(),
                ..SoulConfig::default()
            },
            identity_recovery_state: Some(RecoveryState::Healthy),
            verification_result: Some(VerificationResult {
                status: RegistryStatus::Suspended,
                standing_level: Some("watch".into()),
                reason_code: Some("manual-review".into()),
                verified_at: Some(Utc::now()),
            }),
            verification_provenance: InputProvenance::live("verification.json"),
            generated_at: Utc::now(),
            ..BehaviorInputs::default()
        },
    )
    .expect("bundle should normalize");

    assert_eq!(normalized.compose_mode_hint, Some(ComposeMode::Restricted));
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
