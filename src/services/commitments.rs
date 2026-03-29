use crate::domain::{ComposeMode, NormalizedInputs, RegistryStatus};

const LOW_REPUTATION_THRESHOLD: f32 = 3.0;

#[derive(Debug, Clone, Default)]
pub struct CommitmentsService;

impl CommitmentsService {
    pub fn derive(&self, normalized: &NormalizedInputs, compose_mode: ComposeMode) -> Vec<String> {
        let commitments = normalized
            .upstream
            .identity
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.active_commitments.clone())
            .unwrap_or_default();

        if commitments.is_empty() {
            return Vec::new();
        }

        let mut derived = commitment_context(normalized, compose_mode);
        derived.extend(commitments.into_iter().map(|commitment| {
            format!(
                "{}: {commitment}",
                commitment_prefix(normalized, compose_mode)
            )
        }));
        derived
    }
}

fn commitment_context(normalized: &NormalizedInputs, compose_mode: ComposeMode) -> Vec<String> {
    let mut context = Vec::new();

    match registry_status(normalized) {
        Some(RegistryStatus::Pending) => context.push(
            "Registry standing is pending; do not add new commitments beyond the loaded list."
                .to_owned(),
        ),
        Some(RegistryStatus::Retired) => context.push(
            "Registry standing is retired; treat loaded commitments as historical context only."
                .to_owned(),
        ),
        _ => {}
    }

    if matches!(
        compose_mode,
        ComposeMode::Restricted | ComposeMode::Degraded
    ) {
        context.push(
            "Do not expand loaded commitments without fresh verification or operator approval."
                .to_owned(),
        );
    }

    if matches!(weakest_reputation_score(normalized), Some(score) if score < LOW_REPUTATION_THRESHOLD)
    {
        context.push(
            "Reputation is weak; confirm high-impact commitments before acting on them.".to_owned(),
        );
    }

    context
}

fn commitment_prefix(normalized: &NormalizedInputs, compose_mode: ComposeMode) -> &'static str {
    match registry_status(normalized) {
        Some(RegistryStatus::Pending) => "Pending commitment",
        Some(RegistryStatus::Retired) => "Historical commitment",
        _ if matches!(
            compose_mode,
            ComposeMode::Restricted | ComposeMode::Degraded
        ) =>
        {
            "Constrained commitment"
        }
        _ => "Active commitment",
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
        ReputationSummary, SessionIdentitySnapshot, SoulConfig, VerificationResult,
    };
    use crate::sources::normalize::normalize_inputs;

    use super::CommitmentsService;

    #[test]
    fn derive_shapes_commitments_when_registry_status_is_retired() {
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
                    active_commitments: vec!["follow through".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
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

        let commitments = CommitmentsService.derive(&normalized, ComposeMode::Restricted);

        assert!(
            commitments
                .iter()
                .any(|item| item.contains("historical context only"))
        );
        assert!(
            commitments
                .iter()
                .any(|item| item.contains("Historical commitment: follow through"))
        );
    }

    #[test]
    fn derive_adds_pending_and_low_reputation_commitment_guidance() {
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
                    active_commitments: vec!["follow through".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
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
                    score_total: Some(2.8),
                    score_recent_30d: Some(2.4),
                    last_event_at: None,
                    context: vec!["manual review".to_owned()],
                }),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let commitments = CommitmentsService.derive(&normalized, ComposeMode::Degraded);

        assert!(commitments.iter().any(|item| item.contains("pending")));
        assert!(
            commitments
                .iter()
                .any(|item| item.contains("Reputation is weak"))
        );
        assert!(
            commitments
                .iter()
                .any(|item| item.contains("Pending commitment: follow through"))
        );
    }
}
