use agents_soul::domain::{
    BehaviorInputs, ComposeMode, ComposeRequest, RecoveryState, RegistryStatus, RelationshipMarker,
    SessionIdentitySnapshot, SoulConfig, VerificationResult,
};
use agents_soul::sources::normalize::normalize_inputs;
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
            verification_result: Some(VerificationResult {
                status: RegistryStatus::Active,
                standing_level: Some("good".into()),
                reason_code: None,
                verified_at: Some(Utc::now()),
            }),
            reputation_summary: None,
            generated_at: Utc::now(),
            ..BehaviorInputs::default()
        },
    )
    .expect("bundle should normalize");

    assert_eq!(normalized.agent_id, "alpha");
    assert_eq!(normalized.profile_name, "Alpha Builder");
    assert_eq!(normalized.compose_mode_hint, Some(ComposeMode::Normal));
    assert_eq!(
        normalized
            .identity_snapshot
            .as_ref()
            .expect("identity snapshot should survive")
            .active_commitments,
        vec!["a".to_owned(), "b".to_owned()]
    );
}
