use agents_soul::app::config::WorkspacePaths;
use agents_soul::domain::{
    BehaviorWarning, BehavioralContext, CommunicationStyle, ComposeMode, ComposeRequest,
    OfflineRegistryBehavior, PersonalityProfile, ProvenanceReport, RecoveryState, RegistryStatus,
    RevokedBehavior, SoulConfig, SoulLimits, StatusSummary, TemplateConfig, WarningSeverity,
};

#[test]
fn workspace_paths_match_planned_layout() {
    let paths = WorkspacePaths::new("/tmp/example-soul");

    assert_eq!(
        paths.config_path().to_string_lossy(),
        "/tmp/example-soul/soul.toml"
    );
    assert_eq!(
        paths.adaptation_db_path().to_string_lossy(),
        "/tmp/example-soul/.soul/patterns.sqlite"
    );
    assert_eq!(
        paths.context_cache_path().to_string_lossy(),
        "/tmp/example-soul/.soul/context_cache.json"
    );
    assert_eq!(
        paths.adaptation_log_path().to_string_lossy(),
        "/tmp/example-soul/.soul/adaptation_log.jsonl"
    );
}

#[test]
fn config_defaults_match_reference_semantics() {
    let config = SoulConfig {
        schema_version: 1,
        agent_id: "alpha".to_owned(),
        profile_name: "Alpha Builder".to_owned(),
        ..SoulConfig::default()
    };

    assert_eq!(config.trait_baseline, PersonalityProfile::default());
    assert_eq!(config.communication_style, CommunicationStyle::default());
    assert_eq!(config.limits, SoulLimits::default());
    assert_eq!(config.templates, TemplateConfig::default());
    assert!(config.decision_heuristics.is_empty());
    assert_eq!(
        config.limits.offline_registry_behavior,
        OfflineRegistryBehavior::Cautious
    );
    assert_eq!(config.limits.revoked_behavior, RevokedBehavior::FailClosed);
}

#[test]
fn compose_request_defaults_to_full_domain_context() {
    let request = ComposeRequest::new("alpha", "session-1");

    assert_eq!(request.workspace_id, ".");
    assert_eq!(request.agent_id, "alpha");
    assert!(request.include_reputation);
    assert!(request.include_relationships);
    assert!(request.include_commitments);
}

#[test]
fn behavioral_context_carries_typed_status_information() {
    let context = BehavioralContext {
        schema_version: 1,
        agent_id: "alpha".to_owned(),
        profile_name: "Alpha Builder".to_owned(),
        status_summary: StatusSummary {
            compose_mode: ComposeMode::Restricted,
            identity_loaded: true,
            registry_verified: true,
            registry_status: Some(RegistryStatus::Suspended),
            reputation_loaded: true,
            recovery_state: Some(RecoveryState::Healthy),
        },
        trait_profile: PersonalityProfile::default(),
        communication_rules: vec!["Respond concisely and directly.".to_owned()],
        decision_rules: vec!["Require operator confirmation for risky actions.".to_owned()],
        active_commitments: vec!["Finish contract review.".to_owned()],
        relationship_context: vec!["User prefers direct discussion.".to_owned()],
        adaptive_notes: vec!["Risk tolerance reduced after recent correction events.".to_owned()],
        warnings: vec![BehaviorWarning {
            severity: WarningSeverity::Severe,
            code: "registry_suspended".to_owned(),
            message: "Registry standing is suspended; autonomous behavior must be restricted."
                .to_owned(),
        }],
        system_prompt_prefix: "You are agent alpha.".to_owned(),
        provenance: ProvenanceReport {
            identity_fingerprint: Some("abc123".to_owned()),
            registry_verification_at: None,
            config_hash: "cfg_001".to_owned(),
            adaptation_hash: "adp_001".to_owned(),
            input_hash: "inp_001".to_owned(),
        },
    };

    assert_eq!(context.status_summary.compose_mode, ComposeMode::Restricted);
    assert_eq!(
        context.status_summary.registry_status,
        Some(RegistryStatus::Suspended)
    );
    assert_eq!(context.warnings.len(), 1);
    assert_eq!(context.warnings[0].severity, WarningSeverity::Severe);
}

#[test]
fn compatibility_modules_point_at_canonical_contract_types() {
    let request = agents_soul::domain::compose::ComposeRequest::new("alpha", "session-1");
    let warning = agents_soul::domain::context::BehavioralWarning {
        severity: agents_soul::domain::context::WarningSeverity::Caution,
        code: "degraded".to_owned(),
        message: "Using fallback inputs.".to_owned(),
    };
    let context = agents_soul::domain::context::BehavioralContext {
        warnings: vec![warning],
        ..BehavioralContext::default()
    };

    assert_eq!(request.agent_id, "alpha");
    assert_eq!(context.warnings.len(), 1);
    assert_eq!(
        context.status_summary.compose_mode,
        agents_soul::domain::compose::ComposeMode::BaselineOnly
    );
}
