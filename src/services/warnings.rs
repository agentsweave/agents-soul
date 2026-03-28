use crate::domain::{
    BehaviorWarning, ComposeMode, NormalizedInputs, RecoveryState, RegistryStatus, WarningSeverity,
};

#[derive(Debug, Clone, Default)]
pub struct WarningService;

impl WarningService {
    pub fn derive(
        &self,
        normalized: &NormalizedInputs,
        compose_mode: ComposeMode,
    ) -> Vec<BehaviorWarning> {
        let mut warnings = normalized
            .identity_snapshot
            .as_ref()
            .map(|snapshot| snapshot.warnings.clone())
            .unwrap_or_default();

        if normalized.identity_snapshot.is_none() {
            warnings.push(warning(
                WarningSeverity::Caution,
                "identity_unavailable",
                "Identity snapshot is unavailable; composition is using baseline-only local context.",
            ));
        }

        if normalized.verification_result.is_none() {
            warnings.push(warning(
                WarningSeverity::Important,
                "registry_unavailable",
                "Registry verification is unavailable; offline policy is shaping the compose mode.",
            ));
        }

        if normalized.request.include_reputation && normalized.reputation_summary.is_none() {
            warnings.push(warning(
                WarningSeverity::Info,
                "reputation_unavailable",
                "Registry reputation data is unavailable; reputation shaping was skipped.",
            ));
        }

        if let Some(recovery_state) = normalized
            .identity_snapshot
            .as_ref()
            .map(|snapshot| snapshot.recovery_state)
        {
            match recovery_state {
                RecoveryState::Healthy => {}
                RecoveryState::Recovering => warnings.push(warning(
                    WarningSeverity::Caution,
                    "identity_recovering",
                    "Identity state is recovering; autonomy should stay conservative.",
                )),
                RecoveryState::Degraded => warnings.push(warning(
                    WarningSeverity::Important,
                    "identity_degraded",
                    "Identity state is degraded; autonomy has been reduced.",
                )),
                RecoveryState::Broken => warnings.push(warning(
                    WarningSeverity::Severe,
                    "identity_broken",
                    "Identity state is broken; trust identity-derived context only with caution.",
                )),
            }
        }

        match normalized
            .verification_result
            .as_ref()
            .map(|verification| verification.status)
        {
            Some(RegistryStatus::Suspended) => warnings.push(warning(
                WarningSeverity::Severe,
                "registry_suspended",
                "Registry standing is suspended; autonomous behavior must be restricted.",
            )),
            Some(RegistryStatus::Revoked) => warnings.push(warning(
                WarningSeverity::Severe,
                "registry_revoked",
                "Registry standing is revoked; fail closed and escalate to the operator.",
            )),
            _ => {}
        }

        match compose_mode {
            ComposeMode::Normal => {}
            ComposeMode::BaselineOnly => warnings.push(warning(
                WarningSeverity::Caution,
                "baseline_only",
                "Only the baseline soul profile is active because identity-derived context was not loaded.",
            )),
            ComposeMode::Degraded => warnings.push(warning(
                WarningSeverity::Important,
                "compose_degraded",
                "Composition is degraded; autonomy and confidence should be visibly reduced.",
            )),
            ComposeMode::Restricted => warnings.push(warning(
                WarningSeverity::Severe,
                "compose_restricted",
                "Restricted mode is active; operator confirmation is required for risky actions.",
            )),
            ComposeMode::FailClosed => warnings.push(warning(
                WarningSeverity::Severe,
                "compose_fail_closed",
                "Fail-closed mode is active; do not continue normal operation.",
            )),
        }

        warnings.sort_by(|left, right| {
            (
                severity_rank(left.severity),
                left.code.as_str(),
                left.message.as_str(),
            )
                .cmp(&(
                    severity_rank(right.severity),
                    right.code.as_str(),
                    right.message.as_str(),
                ))
        });
        warnings.dedup_by(|left, right| left.code == right.code && left.message == right.message);
        warnings
    }
}

fn warning(severity: WarningSeverity, code: &str, message: &str) -> BehaviorWarning {
    BehaviorWarning {
        severity,
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

fn severity_rank(severity: WarningSeverity) -> u8 {
    match severity {
        WarningSeverity::Severe => 0,
        WarningSeverity::Important => 1,
        WarningSeverity::Caution => 2,
        WarningSeverity::Info => 3,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::domain::{
        AdaptationState, BehaviorInputs, ComposeMode, ComposeRequest, RecoveryState,
        RegistryStatus, SessionIdentitySnapshot, SoulConfig, VerificationResult, WarningSeverity,
    };
    use crate::sources::normalize::normalize_inputs;

    use super::WarningService;

    #[test]
    fn derive_orders_high_severity_warnings_first() {
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
                    recovery_state: RecoveryState::Broken,
                    active_commitments: Vec::new(),
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: None,
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Revoked,
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

        let warnings = WarningService.derive(&normalized, ComposeMode::FailClosed);
        assert_eq!(
            warnings.first().map(|warning| warning.severity),
            Some(WarningSeverity::Severe)
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code == "registry_revoked")
        );
    }
}
