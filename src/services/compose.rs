use std::time::SystemTime;

use crate::{
    domain::{
        BehaviorInputs, BehaviorWarning, BehavioralContext, CURRENT_SCHEMA_VERSION, ComposeMode,
        ComposeRequest, NormalizedInputs, RegistryStatus, SoulConfig, StatusSummary,
        WarningSeverity,
    },
    services::provenance::ProvenanceService,
    sources::{identity::IdentityReader, normalize::normalize_inputs, registry::RegistryReader},
};

use super::ServiceError;

#[derive(Debug, Clone, Default)]
pub struct ComposeService;

impl ComposeService {
    pub fn compose(&self, request: ComposeRequest) -> Result<BehavioralContext, ServiceError> {
        request.validate().map_err(ServiceError::InvalidRequest)?;
        let config = SoulConfig {
            agent_id: request.agent_id.clone(),
            profile_name: request.agent_id.clone(),
            ..SoulConfig::default()
        };

        let identity_reader = IdentityReader;
        let registry_reader = RegistryReader;

        let identity_snapshot = match identity_reader.read_snapshot(&request) {
            Ok(snapshot) => Some(snapshot),
            Err(crate::domain::SoulError::IdentityUnavailable) => None,
            Err(error) => return Err(ServiceError::InvalidRequest(error)),
        };
        let verification_result = match registry_reader.verify(&request) {
            Ok(verification) => Some(verification),
            Err(crate::domain::SoulError::RegistryUnavailable) => None,
            Err(error) => return Err(ServiceError::InvalidRequest(error)),
        };
        let reputation_summary = match registry_reader.reputation(&request) {
            Ok(reputation) => Some(reputation),
            Err(crate::domain::SoulError::RegistryUnavailable) => None,
            Err(error) => return Err(ServiceError::InvalidRequest(error)),
        };

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                schema_version: CURRENT_SCHEMA_VERSION,
                identity_snapshot,
                verification_result,
                reputation_summary,
                soul_config: config,
                adaptation_state: crate::domain::AdaptationState::default(),
                generated_at: SystemTime::UNIX_EPOCH.into(),
            },
        )
        .map_err(ServiceError::InvalidRequest)?;

        Ok(self.build_context(normalized))
    }

    fn build_context(&self, normalized: NormalizedInputs) -> BehavioralContext {
        let agent_id = normalized.agent_id.clone();
        let profile_name = normalized.profile_name.clone();
        let status_summary = build_status_summary(&normalized);
        let warnings = build_warnings(&normalized, &status_summary);
        let provenance = ProvenanceService.build(&normalized);

        BehavioralContext {
            schema_version: CURRENT_SCHEMA_VERSION,
            agent_id,
            profile_name,
            status_summary: status_summary.clone(),
            trait_profile: normalized.soul_config.trait_baseline.clone(),
            communication_rules: build_communication_rules(&normalized),
            decision_rules: normalized
                .soul_config
                .decision_heuristics
                .iter()
                .filter(|heuristic| heuristic.enabled)
                .map(|heuristic| heuristic.instruction.clone())
                .collect(),
            active_commitments: normalized
                .identity_snapshot
                .as_ref()
                .map(|snapshot| snapshot.active_commitments.clone())
                .unwrap_or_default(),
            relationship_context: normalized
                .identity_snapshot
                .as_ref()
                .map(|snapshot| {
                    snapshot
                        .relationship_markers
                        .iter()
                        .map(|marker| match &marker.note {
                            Some(note) => format!("{}:{} ({note})", marker.subject, marker.marker),
                            None => format!("{}:{}", marker.subject, marker.marker),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            adaptive_notes: normalized.adaptation_state.notes.clone(),
            warnings,
            system_prompt_prefix: build_prompt_prefix(
                &status_summary,
                &normalized.soul_config.profile_name,
            ),
            provenance,
        }
    }
}

fn build_status_summary(normalized: &NormalizedInputs) -> StatusSummary {
    StatusSummary {
        compose_mode: normalized
            .compose_mode_hint
            .unwrap_or(ComposeMode::BaselineOnly),
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

fn build_warnings(
    normalized: &NormalizedInputs,
    status_summary: &StatusSummary,
) -> Vec<BehaviorWarning> {
    let mut warnings = normalized
        .identity_snapshot
        .as_ref()
        .map(|snapshot| snapshot.warnings.clone())
        .unwrap_or_default();

    if normalized.identity_snapshot.is_none() {
        warnings.push(BehaviorWarning {
            severity: WarningSeverity::Caution,
            code: "identity_unavailable".to_owned(),
            message: "Identity snapshot is unavailable; using baseline-only identity context."
                .to_owned(),
        });
    }

    if normalized.verification_result.is_none() {
        warnings.push(BehaviorWarning {
            severity: WarningSeverity::Important,
            code: "registry_unavailable".to_owned(),
            message: "Registry verification is unavailable; composition is operating under offline policy.".to_owned(),
        });
    }

    if normalized.request.include_reputation && normalized.reputation_summary.is_none() {
        warnings.push(BehaviorWarning {
            severity: WarningSeverity::Info,
            code: "reputation_unavailable".to_owned(),
            message: "Registry reputation data is unavailable; reputation shaping was omitted."
                .to_owned(),
        });
    }

    match status_summary.registry_status {
        Some(RegistryStatus::Suspended) => warnings.push(BehaviorWarning {
            severity: WarningSeverity::Severe,
            code: "registry_suspended".to_owned(),
            message: "Registry standing is suspended; autonomous behavior must be restricted."
                .to_owned(),
        }),
        Some(RegistryStatus::Revoked) => warnings.push(BehaviorWarning {
            severity: WarningSeverity::Severe,
            code: "registry_revoked".to_owned(),
            message: "Registry standing is revoked; fail closed and escalate to the operator."
                .to_owned(),
        }),
        _ => {}
    }

    warnings.sort_by(|left, right| {
        (
            warning_rank(left.severity),
            left.code.as_str(),
            left.message.as_str(),
        )
            .cmp(&(
                warning_rank(right.severity),
                right.code.as_str(),
                right.message.as_str(),
            ))
    });
    warnings.dedup_by(|left, right| {
        left.severity == right.severity && left.code == right.code && left.message == right.message
    });
    warnings
}

fn build_communication_rules(normalized: &NormalizedInputs) -> Vec<String> {
    let style = &normalized.soul_config.communication_style;

    vec![
        format!("Default register: {:?}", style.default_register),
        format!("Paragraph budget: {:?}", style.paragraph_budget),
        format!("Question style: {:?}", style.question_style),
        format!("Uncertainty style: {:?}", style.uncertainty_style),
        format!("Feedback style: {:?}", style.feedback_style),
        format!("Conflict style: {:?}", style.conflict_style),
    ]
}

fn build_prompt_prefix(status_summary: &StatusSummary, profile_name: &str) -> String {
    match status_summary.compose_mode {
        ComposeMode::FailClosed => {
            "Registry standing is revoked. Do not operate normally; escalate to the operator.".to_owned()
        }
        ComposeMode::Restricted => {
            "Operate in restricted mode. Ask for operator confirmation before risky or autonomous actions.".to_owned()
        }
        ComposeMode::Degraded => {
            "Operate cautiously. Some upstream identity or registry inputs are unavailable or degraded.".to_owned()
        }
        ComposeMode::BaselineOnly => {
            format!("Use the baseline soul profile for {profile_name}; identity-derived context is unavailable.")
        }
        ComposeMode::Normal => format!("You are {profile_name}. Follow the configured soul profile."),
    }
}

fn warning_rank(severity: WarningSeverity) -> u8 {
    match severity {
        WarningSeverity::Info => 0,
        WarningSeverity::Caution => 1,
        WarningSeverity::Important => 2,
        WarningSeverity::Severe => 3,
    }
}
