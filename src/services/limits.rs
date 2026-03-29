use crate::domain::{ComposeMode, ComposeModeResolver, NormalizedInputs, StatusSummary};

use super::templates::render_builtin_prompt_prefix;

#[derive(Debug, Clone, Default)]
pub struct ComposeModeService;

impl ComposeModeService {
    pub fn resolve(&self, normalized: &NormalizedInputs) -> ComposeMode {
        normalized.compose_mode_hint.unwrap_or_else(|| {
            ComposeModeResolver::resolve(
                normalized
                    .upstream
                    .registry
                    .verification
                    .as_ref()
                    .map(|verification| verification.status),
                normalized.upstream.identity.recovery_state,
                normalized.soul_config.limits.offline_registry_behavior,
            )
        })
    }

    pub fn build_status_summary(
        &self,
        normalized: &NormalizedInputs,
        compose_mode: ComposeMode,
    ) -> StatusSummary {
        StatusSummary {
            compose_mode,
            identity_loaded: normalized.upstream.identity.snapshot.is_some(),
            registry_verified: normalized.upstream.registry.verification.is_some(),
            registry_status: normalized
                .upstream
                .registry
                .verification
                .as_ref()
                .map(|verification| verification.status),
            reputation_loaded: normalized.upstream.registry.reputation.is_some(),
            recovery_state: normalized.upstream.identity.recovery_state,
        }
    }

    pub fn prompt_prefix(
        &self,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> String {
        render_builtin_prompt_prefix(compose_mode, profile_name, max_chars)
    }
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
        assert_eq!(service.resolve(&normalized), ComposeMode::BaselineOnly);
    }

    #[test]
    fn resolve_caps_offline_fail_closed_to_baseline_only_without_identity() {
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
        assert_eq!(service.resolve(&normalized), ComposeMode::BaselineOnly);
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

    #[test]
    fn prompt_prefix_uses_plan_aligned_fail_closed_guidance() {
        let prefix = ComposeModeService.prompt_prefix(ComposeMode::FailClosed, "Alpha", 512);

        assert!(prefix.starts_with("FAIL-CLOSED: identity revoked."));
        assert!(prefix.contains("Do not take on new commitments."));
        assert!(prefix.contains("Do not claim registry validity."));
    }

    #[test]
    fn prompt_prefix_uses_plan_aligned_restricted_guidance() {
        let prefix = ComposeModeService.prompt_prefix(ComposeMode::Restricted, "Alpha", 512);

        assert!(prefix.starts_with("RESTRICTED: identity suspended."));
        assert!(prefix.contains("Lower initiative."));
        assert!(prefix.contains("Request operator confirmation"));
    }

    #[test]
    fn build_status_summary_preserves_upstream_visibility_for_degraded_inputs() {
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
                identity_recovery_state: Some(RecoveryState::Recovering),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let service = ComposeModeService;
        let compose_mode = service.resolve(&normalized);
        let summary = service.build_status_summary(&normalized, compose_mode);

        assert_eq!(summary.compose_mode, ComposeMode::Degraded);
        assert!(!summary.identity_loaded);
        assert!(!summary.registry_verified);
        assert!(!summary.reputation_loaded);
        assert_eq!(summary.registry_status, None);
        assert_eq!(summary.recovery_state, Some(RecoveryState::Recovering));
    }

    #[test]
    fn build_status_summary_marks_loaded_upstream_inputs() {
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
                    display_name: Some("Alpha".into()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: Vec::new(),
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: None,
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Active,
                    standing_level: Some("good".into()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                reputation_summary: Some(crate::domain::ReputationSummary {
                    score_total: Some(4.8),
                    score_recent_30d: Some(4.6),
                    last_event_at: Some(Utc::now()),
                    context: vec!["steady".into()],
                }),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let service = ComposeModeService;
        let compose_mode = service.resolve(&normalized);
        let summary = service.build_status_summary(&normalized, compose_mode);

        assert_eq!(summary.compose_mode, ComposeMode::Normal);
        assert!(summary.identity_loaded);
        assert!(summary.registry_verified);
        assert!(summary.reputation_loaded);
        assert_eq!(summary.registry_status, Some(RegistryStatus::Active));
        assert_eq!(summary.recovery_state, Some(RecoveryState::Healthy));
    }
}
