use std::time::SystemTime;

use crate::{
    adaptation::read_workspace_effective_overrides,
    app::{
        config::{WorkspacePaths, load_soul_config},
        deps::SourceDependencies,
    },
    domain::{
        BehaviorInputs, BehavioralContext, CURRENT_SCHEMA_VERSION, ComposeMode, ComposeRequest,
        NormalizedInputs, SoulConfig, StatusSummary,
    },
    services::{
        commitments::CommitmentsService, communication::CommunicationRulesService,
        decision_rules::DecisionRulesService, limits::ComposeModeService,
        profile::EffectiveProfileService, provenance::ProvenanceService,
        relationships::RelationshipsService, warnings::WarningService,
    },
    sources::normalize::normalize_inputs,
};

use super::ServiceError;

#[derive(Debug, Clone, Default)]
pub struct ComposeService;

impl ComposeService {
    pub fn compose(
        &self,
        deps: &SourceDependencies,
        request: ComposeRequest,
    ) -> Result<BehavioralContext, ServiceError> {
        request.validate()?;
        let config = load_config_for_request(&request)?;
        let effective_overrides =
            read_workspace_effective_overrides(&request.workspace_id, &config, &request.agent_id)?;

        let identity_selection = deps.identity.load(&request, &config)?;
        let verification_selection = deps.registry.load_verification(&request)?;
        let reputation_selection = deps.registry.load_reputation(&request)?;
        let mut reader_warnings = identity_selection.warnings.clone();
        reader_warnings.extend(verification_selection.warnings.clone());
        reader_warnings.extend(reputation_selection.warnings.clone());

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                schema_version: CURRENT_SCHEMA_VERSION,
                identity_snapshot: identity_selection.value,
                identity_provenance: identity_selection.provenance,
                verification_result: verification_selection.value,
                verification_provenance: verification_selection.provenance,
                reputation_summary: reputation_selection.value,
                reputation_provenance: reputation_selection.provenance,
                soul_config: config,
                adaptation_state: effective_overrides.adaptation_state,
                reader_warnings,
                generated_at: SystemTime::UNIX_EPOCH.into(),
            },
        )?;

        Ok(self.build_context(normalized))
    }

    fn build_context(&self, normalized: NormalizedInputs) -> BehavioralContext {
        let compose_mode = ComposeModeService.resolve(&normalized);
        let status_summary = ComposeModeService.build_status_summary(&normalized, compose_mode);

        match compose_mode {
            ComposeMode::FailClosed => self.build_fail_closed_context(normalized, status_summary),
            ComposeMode::Restricted => self.build_restricted_context(normalized, status_summary),
            _ => self.render_context(normalized, status_summary, compose_mode),
        }
    }

    fn build_fail_closed_context(
        &self,
        normalized: NormalizedInputs,
        status_summary: StatusSummary,
    ) -> BehavioralContext {
        let profile_name = normalized.profile_name.clone();
        let prompt_prefix = ComposeModeService.prompt_prefix(
            ComposeMode::FailClosed,
            &profile_name,
            normalized.soul_config.limits.max_prompt_prefix_chars,
        );
        let fail_closed_inputs = fail_closed_inputs(&normalized);

        BehavioralContext {
            schema_version: CURRENT_SCHEMA_VERSION,
            agent_id: normalized.agent_id.clone(),
            profile_name,
            status_summary,
            trait_profile: EffectiveProfileService
                .derive(&fail_closed_inputs, ComposeMode::FailClosed),
            communication_rules: vec![
                "State the fail-closed state plainly.".to_owned(),
                "Do not present yourself as an active verified agent.".to_owned(),
                "Ask for operator intervention before any further action.".to_owned(),
                "Do not take on new commitments or claim registry validity.".to_owned(),
            ],
            decision_rules: vec![
                "Do not continue normal autonomous operation.".to_owned(),
                "Decline to take new commitments until the operator restores registry standing."
                    .to_owned(),
            ],
            active_commitments: Vec::new(),
            relationship_context: Vec::new(),
            adaptive_notes: Vec::new(),
            warnings: WarningService.derive(&normalized, ComposeMode::FailClosed),
            system_prompt_prefix: prompt_prefix,
            provenance: ProvenanceService.build(&normalized),
        }
    }

    fn build_restricted_context(
        &self,
        normalized: NormalizedInputs,
        status_summary: StatusSummary,
    ) -> BehavioralContext {
        self.render_context(normalized, status_summary, ComposeMode::Restricted)
    }

    fn render_context(
        &self,
        normalized: NormalizedInputs,
        status_summary: StatusSummary,
        compose_mode: ComposeMode,
    ) -> BehavioralContext {
        let profile_name = normalized.profile_name.clone();
        let prompt_prefix = ComposeModeService.prompt_prefix(
            compose_mode,
            &profile_name,
            normalized.soul_config.limits.max_prompt_prefix_chars,
        );

        BehavioralContext {
            schema_version: CURRENT_SCHEMA_VERSION,
            agent_id: normalized.agent_id.clone(),
            profile_name,
            status_summary,
            trait_profile: EffectiveProfileService.derive(&normalized, compose_mode),
            communication_rules: CommunicationRulesService.derive(&normalized, compose_mode),
            decision_rules: DecisionRulesService.derive(&normalized, compose_mode),
            active_commitments: CommitmentsService.derive(&normalized),
            relationship_context: RelationshipsService.derive(&normalized),
            adaptive_notes: normalized.adaptation_state.notes.clone(),
            warnings: WarningService.derive(&normalized, compose_mode),
            system_prompt_prefix: prompt_prefix,
            provenance: ProvenanceService.build(&normalized),
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

fn fail_closed_inputs(normalized: &NormalizedInputs) -> NormalizedInputs {
    let mut fail_closed = normalized.clone();
    fail_closed.adaptation_state = Default::default();
    fail_closed.soul_config.adaptation.enabled = false;
    fail_closed
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
        domain::{
            AdaptationConfig, AdaptationState, BehaviorInputs, ComposeMode, ComposeRequest,
            DecisionHeuristic, PersonalityOverride, RecoveryState, RegistryStatus,
            RelationshipMarker, SessionIdentitySnapshot, SoulConfig, VerificationResult,
        },
        sources::normalize::normalize_inputs,
        storage::sqlite::{AdaptationStateRecord, open_database, upsert_adaptation_state},
    };

    use super::ComposeService;

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

        let context = ComposeService.compose(
            &crate::app::deps::SourceDependencies::default(),
            ComposeRequest {
                workspace_id: workspace.display().to_string(),
                agent_id: "agent.alpha".to_owned(),
                session_id: "session.alpha".to_owned(),
                identity_snapshot_path: None,
                registry_verification_path: None,
                registry_reputation_path: None,
                include_reputation: true,
                include_relationships: true,
                include_commitments: true,
            },
        )?;

        assert_eq!(context.trait_profile.verbosity, 0.49);
        assert_eq!(
            context.communication_rules,
            vec![
                "Call out degraded or missing upstream context before acting on uncertain assumptions."
                    .to_owned(),
                "Reduce autonomous initiative until identity and registry inputs are healthy again."
                    .to_owned(),
                "Use a professional-direct register.".to_owned(),
                "Keep responses within a long paragraph budget.".to_owned(),
                "Questions: ask a single clarifying question only when needed.".to_owned(),
                "Uncertainty: state uncertainty explicitly and keep it bounded.".to_owned(),
                "Feedback: be frank.".to_owned(),
                "Conflict handling: stay firm and respectful.".to_owned(),
            ]
        );
        assert_eq!(
            context.decision_rules,
            vec![
                "Prefer reversible actions and verification steps while upstream context is degraded."
                    .to_owned(),
                "Use adapted risk review.".to_owned()
            ]
        );
        assert_eq!(
            context.adaptive_notes,
            vec!["adapted note".to_owned(), "alpha note".to_owned()]
        );

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn revoked_input_short_circuits_to_minimal_fail_closed_context() {
        let request = ComposeRequest::new("agent.alpha", "session.alpha");
        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.adaptation.enabled = true;

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "agent.alpha".to_owned(),
                    display_name: Some("Alpha".to_owned()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["protect the operator".to_owned()],
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
                    status: RegistryStatus::Revoked,
                    standing_level: Some("revoked".to_owned()),
                    reason_code: Some("policy".to_owned()),
                    verified_at: Some(Utc::now()),
                }),
                adaptation_state: AdaptationState {
                    trait_overrides: PersonalityOverride {
                        initiative: 0.30,
                        verbosity: 0.40,
                        ..PersonalityOverride::default()
                    },
                    notes: vec!["adapted note".to_owned()],
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let context = ComposeService.build_context(normalized);

        assert_eq!(context.status_summary.compose_mode, ComposeMode::FailClosed);
        assert!(
            context
                .system_prompt_prefix
                .starts_with("Identity revoked.")
        );
        assert_eq!(context.communication_rules.len(), 4);
        assert_eq!(context.decision_rules.len(), 2);
        assert!(context.active_commitments.is_empty());
        assert!(context.relationship_context.is_empty());
        assert!(context.adaptive_notes.is_empty());
        assert!(context.trait_profile.initiative <= 0.05);
        assert!(context.trait_profile.verbosity <= 0.25);
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.code == "registry_revoked")
        );
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
