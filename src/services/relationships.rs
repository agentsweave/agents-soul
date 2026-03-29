use crate::domain::{ComposeMode, NormalizedInputs, RegistryStatus, RelationshipMarker};

const LOW_REPUTATION_THRESHOLD: f32 = 3.0;

#[derive(Debug, Clone, Default)]
pub struct RelationshipsService;

impl RelationshipsService {
    pub fn derive(&self, normalized: &NormalizedInputs, compose_mode: ComposeMode) -> Vec<String> {
        let Some(snapshot) = normalized.upstream.identity.snapshot.as_ref() else {
            return Vec::new();
        };

        if snapshot.relationship_markers.is_empty() {
            return Vec::new();
        }

        let mut derived = relationship_context(normalized, compose_mode);
        derived.extend(
            snapshot
                .relationship_markers
                .iter()
                .map(|marker| format_relationship(marker, normalized, compose_mode)),
        );
        derived
    }
}

fn relationship_context(normalized: &NormalizedInputs, compose_mode: ComposeMode) -> Vec<String> {
    let mut context = Vec::new();

    match registry_status(normalized) {
        Some(RegistryStatus::Pending) => context.push(
            "Registry standing is pending; relationship markers are provisional until activation."
                .to_owned(),
        ),
        Some(RegistryStatus::Retired) => context.push(
            "Registry standing is retired; relationship markers are historical context only."
                .to_owned(),
        ),
        _ => {}
    }

    if matches!(compose_mode, ComposeMode::Restricted) {
        context.push(
            "Relationship markers do not override restricted-mode approval requirements."
                .to_owned(),
        );
    }

    if matches!(weakest_reputation_score(normalized), Some(score) if score < LOW_REPUTATION_THRESHOLD)
    {
        context.push(
            "Reputation is weak; relationship markers do not substitute for fresh verification."
                .to_owned(),
        );
    }

    context
}

fn format_relationship(
    marker: &RelationshipMarker,
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> String {
    let label = match registry_status(normalized) {
        Some(RegistryStatus::Pending) => "Provisional relationship",
        Some(RegistryStatus::Retired) => "Historical relationship",
        _ if matches!(compose_mode, ComposeMode::Restricted) => "Restricted relationship",
        _ => "Relationship",
    };

    match &marker.note {
        Some(note) => format!("{label}: {} -> {} ({note})", marker.subject, marker.marker),
        None => format!("{label}: {} -> {}", marker.subject, marker.marker),
    }
}

fn registry_status(normalized: &NormalizedInputs) -> Option<RegistryStatus> {
    normalized
        .upstream
        .registry
        .verification
        .as_ref()
        .map(|verification| verification.status)
}

fn weakest_reputation_score(normalized: &NormalizedInputs) -> Option<f32> {
    let reputation = normalized.upstream.registry.reputation.as_ref()?;
    match (reputation.score_recent_30d, reputation.score_total) {
        (Some(recent), Some(total)) => Some(recent.min(total)),
        (Some(recent), None) => Some(recent),
        (None, Some(total)) => Some(total),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::domain::{
        BehaviorInputs, ComposeMode, ComposeRequest, RecoveryState, RegistryStatus,
        RelationshipMarker, ReputationSummary, SessionIdentitySnapshot, SoulConfig,
        VerificationResult,
    };
    use crate::sources::normalize::normalize_inputs;

    use super::RelationshipsService;

    #[test]
    fn derive_marks_relationships_as_historical_when_registry_status_is_retired() {
        let request = ComposeRequest::new("alpha", "session-1");
        let config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "alpha".into(),
                    display_name: None,
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: Vec::new(),
                    durable_preferences: Vec::new(),
                    relationship_markers: vec![RelationshipMarker {
                        subject: "operator".to_owned(),
                        marker: "trusted".to_owned(),
                        note: Some("primary owner".to_owned()),
                    }],
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: None,
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Retired,
                    standing_level: Some("historical".to_owned()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let relationships = RelationshipsService.derive(&normalized, ComposeMode::Restricted);

        assert!(
            relationships
                .iter()
                .any(|item| item.contains("historical context only"))
        );
        assert!(relationships.iter().any(|item| {
            item.contains("Historical relationship: operator -> trusted (primary owner)")
        }));
    }

    #[test]
    fn derive_adds_pending_and_low_reputation_relationship_guidance() {
        let request = ComposeRequest::new("alpha", "session-1");
        let config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "alpha".into(),
                    display_name: None,
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: Vec::new(),
                    durable_preferences: Vec::new(),
                    relationship_markers: vec![RelationshipMarker {
                        subject: "operator".to_owned(),
                        marker: "trusted".to_owned(),
                        note: None,
                    }],
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: None,
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Pending,
                    standing_level: Some("probationary".to_owned()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                reputation_summary: Some(ReputationSummary {
                    score_total: Some(2.9),
                    score_recent_30d: Some(2.3),
                    last_event_at: None,
                    context: vec!["manual review".to_owned()],
                }),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let relationships = RelationshipsService.derive(&normalized, ComposeMode::Restricted);

        assert!(relationships.iter().any(|item| item.contains("pending")));
        assert!(
            relationships
                .iter()
                .any(|item| item.contains("Reputation is weak"))
        );
        assert!(
            relationships
                .iter()
                .any(|item| item.contains("Provisional relationship: operator -> trusted"))
        );
    }
}
