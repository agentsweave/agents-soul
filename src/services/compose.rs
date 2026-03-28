use std::time::SystemTime;

use crate::{
    adaptation::{materialize_effective_overrides, read_workspace_adaptation_state},
    app::config::{WorkspacePaths, load_soul_config},
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
        request.validate()?;
        let config = load_config_for_request(&request)?;
        let stored_adaptation =
            read_workspace_adaptation_state(&request.workspace_id, &request.agent_id)?;
        let effective_overrides =
            materialize_effective_overrides(&config, stored_adaptation.as_ref());
        let mut effective_config = config.clone();
        effective_config.trait_baseline = effective_overrides.trait_profile;
        effective_config.communication_style = effective_overrides.communication_style;
        effective_config.decision_heuristics = effective_overrides.decision_heuristics;

        let identity_reader = IdentityReader;
        let registry_reader = RegistryReader;

        let identity_snapshot = match identity_reader.read_snapshot(&request) {
            Ok(snapshot) => Some(snapshot),
            Err(crate::domain::SoulError::IdentityUnavailable) => None,
            Err(error) => return Err(error),
        };
        let verification_result = match registry_reader.verify(&request) {
            Ok(verification) => Some(verification),
            Err(crate::domain::SoulError::RegistryUnavailable) => None,
            Err(error) => return Err(error),
        };
        let reputation_summary = match registry_reader.reputation(&request) {
            Ok(reputation) => Some(reputation),
            Err(crate::domain::SoulError::RegistryUnavailable) => None,
            Err(error) => return Err(error),
        };

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                schema_version: CURRENT_SCHEMA_VERSION,
                identity_snapshot,
                verification_result,
                reputation_summary,
                soul_config: effective_config,
                adaptation_state: effective_overrides.adaptation_state,
                generated_at: SystemTime::UNIX_EPOCH.into(),
            },
        )?;

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

fn load_config_for_request(request: &ComposeRequest) -> Result<SoulConfig, ServiceError> {
    let config_path = WorkspacePaths::new(&request.workspace_id).config_path();
    if config_path.is_file() {
        return load_soul_config(&request.workspace_id);
    }

    Ok(SoulConfig {
        agent_id: request.agent_id.clone(),
        profile_name: request.agent_id.clone(),
        ..SoulConfig::default()
    })
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
            message:
                "Registry verification is unavailable; composition is operating under offline policy."
                    .to_owned(),
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
            "Registry standing is revoked. Do not operate normally; escalate to the operator."
                .to_owned()
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

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use crate::{
        domain::{AdaptationConfig, DecisionHeuristic, SoulConfig},
        storage::sqlite::{AdaptationStateRecord, open_database, upsert_adaptation_state},
    };

    use super::{ComposeRequest, ComposeService};

    #[test]
    fn compose_uses_effective_adaptive_overrides_from_workspace_storage()
    -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-adaptation");
        fs::create_dir_all(&workspace)?;

        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.adaptation = AdaptationConfig {
            enabled: true,
            learning_window_days: 30,
            min_interactions_for_adapt: 1,
        };
        config.decision_heuristics = vec![DecisionHeuristic {
            heuristic_id: "review-risk".to_owned(),
            title: "Review Risk".to_owned(),
            priority: 2,
            trigger: "review".to_owned(),
            instruction: "Use the baseline rule.".to_owned(),
            enabled: true,
            ..DecisionHeuristic::default()
        }];
        fs::write(workspace.join("soul.toml"), toml::to_string(&config)?)?;

        let conn = open_database(
            crate::app::config::WorkspacePaths::new(&workspace).adaptation_db_path(),
        )?;
        upsert_adaptation_state(
            &conn,
            &AdaptationStateRecord {
                agent_id: "agent.alpha".to_owned(),
                trait_overrides_json: r#"{"verbosity":0.25}"#.to_owned(),
                communication_overrides_json: r#"{"paragraph_budget":"long"}"#.to_owned(),
                heuristic_overrides_json: r#"[{"heuristic_id":"review-risk","priority_delta":4,"replacement_instruction":"Use adapted risk review.","enabled":true}]"#.to_owned(),
                notes_json: r#"["adapted note","adapted note","alpha note"]"#.to_owned(),
                evidence_window_size: 10,
                interaction_count: 4,
                last_interaction_at: Some(test_timestamp(2026, 3, 29, 2, 10, 0)?),
                last_reset_at: None,
                updated_at: test_timestamp(2026, 3, 29, 2, 15, 0)?,
            },
        )?;

        let context = ComposeService.compose(ComposeRequest {
            workspace_id: workspace.display().to_string(),
            agent_id: "agent.alpha".to_owned(),
            session_id: "session.alpha".to_owned(),
            include_reputation: true,
            include_relationships: true,
            include_commitments: true,
        })?;

        assert_eq!(context.trait_profile.verbosity, 0.49);
        assert_eq!(
            context.communication_rules,
            vec![
                "Default register: ProfessionalDirect".to_owned(),
                "Paragraph budget: Long".to_owned(),
                "Question style: SingleClarifierWhenNeeded".to_owned(),
                "Uncertainty style: ExplicitAndBounded".to_owned(),
                "Feedback style: Frank".to_owned(),
                "Conflict style: FirmRespectful".to_owned(),
            ]
        );
        assert_eq!(
            context.decision_rules,
            vec!["Use adapted risk review.".to_owned()]
        );
        assert_eq!(
            context.adaptive_notes,
            vec!["adapted note".to_owned(), "alpha note".to_owned()]
        );

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    fn test_workspace(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}"))
    }

    fn cleanup_workspace(workspace: &PathBuf) -> Result<(), Box<dyn Error>> {
        if workspace.exists() {
            fs::remove_dir_all(workspace)?;
        }
        Ok(())
    }

    fn test_timestamp(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Result<chrono::DateTime<Utc>, Box<dyn Error>> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| std::io::Error::other("invalid UTC test timestamp").into())
    }
}
