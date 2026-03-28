use crate::domain::{
    ComposeMode, NormalizedInputs, OfflineRegistryBehavior, RecoveryState, RegistryStatus,
    StatusSummary,
};

#[derive(Debug, Clone, Default)]
pub struct ComposeModeService;

impl ComposeModeService {
    pub fn resolve(&self, normalized: &NormalizedInputs) -> ComposeMode {
        normalized
            .compose_mode_hint
            .unwrap_or_else(|| derive_mode(normalized))
    }

    pub fn build_status_summary(
        &self,
        normalized: &NormalizedInputs,
        compose_mode: ComposeMode,
    ) -> StatusSummary {
        StatusSummary {
            compose_mode,
            identity_loaded: normalized.identity_snapshot.is_some(),
            registry_verified: normalized.verification_result.is_some(),
            registry_status: normalized
                .verification_result
                .as_ref()
                .map(|verification| verification.status),
            reputation_loaded: normalized.reputation_summary.is_some(),
            recovery_state: normalized
                .identity_snapshot
                .as_ref()
                .map(|snapshot| snapshot.recovery_state),
        }
    }

    pub fn prompt_prefix(
        &self,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> String {
        let prefix = match compose_mode {
            ComposeMode::FailClosed => {
                "Registry standing does not permit normal operation. Do not act autonomously; explain the restriction and escalate to the operator."
                    .to_owned()
            }
            ComposeMode::Restricted => {
                "Operate in restricted mode. Ask for operator confirmation before risky, stateful, or autonomous actions."
                    .to_owned()
            }
            ComposeMode::Degraded => {
                "Operate cautiously. Upstream identity or registry inputs are degraded, so autonomy and confidence must be reduced."
                    .to_owned()
            }
            ComposeMode::BaselineOnly => format!(
                "Use the baseline soul profile for {profile_name}. Do not invent identity-derived commitments or relationship context that was not loaded."
            ),
            ComposeMode::Normal => {
                format!("You are {profile_name}. Follow the configured soul profile.")
            }
        };

        truncate(prefix, max_chars)
    }
}

fn derive_mode(normalized: &NormalizedInputs) -> ComposeMode {
    match normalized
        .verification_result
        .as_ref()
        .map(|verification| verification.status)
    {
        Some(RegistryStatus::Revoked) => ComposeMode::FailClosed,
        Some(RegistryStatus::Suspended) => ComposeMode::Restricted,
        Some(_) => match normalized
            .identity_snapshot
            .as_ref()
            .map(|snapshot| snapshot.recovery_state)
        {
            Some(RecoveryState::Broken)
            | Some(RecoveryState::Degraded)
            | Some(RecoveryState::Recovering) => ComposeMode::Degraded,
            Some(RecoveryState::Healthy) => ComposeMode::Normal,
            None => ComposeMode::BaselineOnly,
        },
        None => match normalized.soul_config.limits.offline_registry_behavior {
            OfflineRegistryBehavior::Cautious => ComposeMode::Degraded,
            OfflineRegistryBehavior::BaselineOnly => ComposeMode::BaselineOnly,
            OfflineRegistryBehavior::FailClosed => ComposeMode::FailClosed,
        },
    }
}

fn truncate(mut value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    value = value.chars().take(max_chars).collect();
    value.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::domain::{
        AdaptationState, BehaviorInputs, ComposeMode, ComposeRequest, OfflineRegistryBehavior,
        RecoveryState, RegistryStatus, SessionIdentitySnapshot, SoulConfig, VerificationResult,
    };
    use crate::sources::normalize::normalize_inputs;

    use super::ComposeModeService;

    #[test]
    fn resolve_uses_offline_fail_closed_policy() {
        let request = ComposeRequest::new("alpha", "session-1");
        let mut config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        config.limits.offline_registry_behavior = OfflineRegistryBehavior::FailClosed;

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let service = ComposeModeService;
        assert_eq!(service.resolve(&normalized), ComposeMode::FailClosed);
    }

    #[test]
    fn resolve_restricts_suspended_registry() {
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
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: None,
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Suspended,
                    standing_level: None,
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                adaptation_state: AdaptationState::default(),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let service = ComposeModeService;
        assert_eq!(service.resolve(&normalized), ComposeMode::Restricted);
    }
}
