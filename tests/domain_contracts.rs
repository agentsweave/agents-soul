use agents_soul::{
    BehaviorWarning, BehavioralContext, ComposeMode, ComposeRequest, RecoveryState, RegisterStyle,
    RegistryStatus, SoulConfig, StatusSummary, WarningSeverity,
};
use chrono::Utc;

#[test]
fn reference_toml_parses_with_defaults() {
    let config = toml::from_str::<SoulConfig>(
        r#"
schema_version = 1
agent_id = "alpha"
profile_name = "Alpha Builder"

[trait_baseline]
openness = 0.72
conscientiousness = 0.90
initiative = 0.84
directness = 0.81
warmth = 0.42
risk_tolerance = 0.28
verbosity = 0.34
formality = 0.71

[communication_style]
default_register = "professional-direct"
paragraph_budget = "short"
question_style = "single-clarifier-when-needed"
uncertainty_style = "explicit-and-bounded"
feedback_style = "frank"
conflict_style = "firm-respectful"

[limits]
max_trait_drift = 0.15
max_prompt_prefix_chars = 4000
max_adaptive_rules = 24
offline_registry_behavior = "cautious"
revoked_behavior = "fail-closed"

[sources]
identity_workspace = "~/.agents/alpha"
registry_url = "http://127.0.0.1:7700"
"#,
    )
    .expect("reference config should parse")
    .finalize()
    .expect("reference config should finalize");

    assert_eq!(
        config.communication_style.default_register,
        RegisterStyle::ProfessionalDirect
    );
    assert!(config.decision_heuristics.is_empty());
    assert_eq!(config.templates.prompt_prefix_template, "prompt-prefix");
    assert_eq!(config.sources.registry_agent_id, "alpha");
    assert!(config.adaptation.enabled);
}

#[test]
fn config_validation_rejects_out_of_range_traits() {
    let mut config = SoulConfig::default();
    config.trait_baseline.verbosity = 1.5;

    let error = config.validate().expect_err("config should be rejected");
    assert!(error.to_string().contains("verbosity"));
}

#[test]
fn compose_request_requires_stable_identity_fields() {
    let request = ComposeRequest {
        workspace_id: String::new(),
        agent_id: "alpha".into(),
        session_id: "session-1".into(),
        include_reputation: true,
        include_relationships: true,
        include_commitments: true,
    };

    let error = request.validate().expect_err("empty workspace should fail");
    assert!(error.to_string().contains("workspace_id"));
}

#[test]
fn behavioral_context_serializes_mode_and_structured_warnings() {
    let context = BehavioralContext {
        schema_version: 1,
        agent_id: "alpha".into(),
        profile_name: "Alpha Builder".into(),
        status_summary: StatusSummary {
            compose_mode: ComposeMode::Restricted,
            identity_loaded: true,
            registry_verified: true,
            registry_status: Some(RegistryStatus::Suspended),
            reputation_loaded: false,
            recovery_state: Some(RecoveryState::Healthy),
        },
        trait_profile: SoulConfig::default().trait_baseline,
        communication_rules: vec!["Respond concisely and directly.".into()],
        decision_rules: vec!["Lower initiative under suspension.".into()],
        active_commitments: vec![],
        relationship_context: vec![],
        adaptive_notes: vec![],
        warnings: vec![BehaviorWarning {
            severity: WarningSeverity::Severe,
            code: "registry-suspended".into(),
            message: "Restricted mode is active.".into(),
        }],
        system_prompt_prefix: "You are alpha.".into(),
        provenance: agents_soul::ProvenanceReport {
            identity_fingerprint: Some("abc123".into()),
            registry_verification_at: Some(Utc::now()),
            config_hash: "cfg".into(),
            adaptation_hash: "adp".into(),
            input_hash: "inp".into(),
        },
    };

    let json = serde_json::to_value(&context).expect("behavioral context should serialize");
    assert_eq!(json["status_summary"]["compose_mode"], "restricted");
    assert_eq!(json["warnings"][0]["severity"], "severe");
    assert_eq!(json["warnings"][0]["code"], "registry-suspended");
}
