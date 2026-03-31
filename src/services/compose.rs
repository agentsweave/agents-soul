use crate::{
    adaptation::EffectiveOverrideSet,
    app::{config::WorkspacePaths, deps::AppDeps},
    domain::{
        BehaviorInputs, BehaviorWarning, BehavioralContext, CURRENT_SCHEMA_VERSION, ComposeMode,
        ComposeRequest, IdentifySignals, InputProvenance, InputSourceKind, NormalizedInputs,
        ReputationSummary, SessionIdentitySnapshot, StatusSummary, VerificationResult,
        WarningSeverity,
    },
    services::{
        commitments::CommitmentsService, communication::CommunicationRulesService,
        decision_rules::DecisionRulesService, limits::ComposeModeService,
        profile::EffectiveProfileService, provenance::ProvenanceService,
        relationships::RelationshipsService, warnings::WarningService,
    },
    sources::{
        ReaderSelection,
        cache::{
            CachedFreshness, CachedInputs, cache_stale_warning, read_cached_inputs,
            write_cached_inputs,
        },
        normalize::normalize_inputs,
    },
};

use super::ServiceError;

#[derive(Debug, Clone, Default)]
pub struct ComposeService;

#[derive(Debug, Clone)]
pub struct ComposeArtifacts {
    pub normalized: NormalizedInputs,
    pub effective_overrides: EffectiveOverrideSet,
    pub context: BehavioralContext,
}

#[derive(Debug, Clone)]
struct PreparedComposeInputs {
    normalized: NormalizedInputs,
    effective_overrides: EffectiveOverrideSet,
}

impl ComposeService {
    pub fn compose(
        &self,
        deps: &AppDeps,
        request: ComposeRequest,
    ) -> Result<BehavioralContext, ServiceError> {
        self.compose_artifacts(deps, request)
            .map(|artifacts| artifacts.context)
    }

    pub fn compose_artifacts(
        &self,
        deps: &AppDeps,
        request: ComposeRequest,
    ) -> Result<ComposeArtifacts, ServiceError> {
        let prepared = self.prepare_inputs(deps, &request)?;
        let context = self.build_context(deps, prepared.normalized.clone())?;

        Ok(ComposeArtifacts {
            normalized: prepared.normalized,
            effective_overrides: prepared.effective_overrides,
            context,
        })
    }

    fn prepare_inputs(
        &self,
        deps: &AppDeps,
        request: &ComposeRequest,
    ) -> Result<PreparedComposeInputs, ServiceError> {
        request.validate()?;
        let config = deps.load_soul_config(&request.workspace_id)?;
        let effective_overrides =
            deps.load_effective_overrides(&request.workspace_id, &config, &request.agent_id)?;
        let config_hash = deps.provenance_hasher().config_hash(&config)?;
        let adaptation_hash = deps
            .provenance_hasher()
            .adaptation_hash(&effective_overrides.adaptation_state)?;

        let mut identity_selection = deps.load_identify_signals(&request, &config)?;
        let mut verification_selection = deps.load_registry_verification(&request)?;
        let mut reputation_selection = deps.load_registry_reputation(&request)?;
        invalidate_stale_cache_backed_selections(
            &request,
            &config_hash,
            &adaptation_hash,
            &mut identity_selection,
            &mut verification_selection,
            &mut reputation_selection,
        )?;
        let identity_snapshot = identity_selection
            .value
            .as_ref()
            .and_then(|signals| signals.snapshot.clone());
        let identity_recovery_state = identity_selection
            .value
            .as_ref()
            .and_then(|signals| signals.recovery_state);
        let verification_result = verification_selection.value.clone();
        let reputation_summary = reputation_selection.value.clone();

        let mut reader_warnings = identity_selection.warnings.clone();
        reader_warnings.extend(verification_selection.warnings.clone());
        reader_warnings.extend(reputation_selection.warnings.clone());
        if should_refresh_context_cache(
            identity_selection.provenance.source,
            verification_selection.provenance.source,
            reputation_selection.provenance.source,
        ) {
            let freshness = Some(CachedFreshness {
                config_hash: Some(config_hash.clone()),
                adaptation_hash: Some(adaptation_hash.clone()),
                identity_fingerprint: cache_identity_fingerprint(deps, identity_snapshot.as_ref())?,
                registry_verification_at: verification_result
                    .as_ref()
                    .and_then(|verification| verification.verified_at),
            });

            if let Err(error) = write_cached_inputs(
                request,
                &CachedInputs {
                    cache_key: None,
                    freshness,
                    identity_snapshot: identity_snapshot.clone(),
                    verification_result: verification_result.clone(),
                    reputation_summary: reputation_summary.clone(),
                },
            ) {
                reader_warnings.push(cache_write_warning(request, error));
            }
        }

        let normalized = normalize_inputs(
            request,
            BehaviorInputs {
                schema_version: CURRENT_SCHEMA_VERSION,
                identity_recovery_state,
                identity_snapshot,
                identity_provenance: identity_selection.provenance,
                verification_result,
                verification_provenance: verification_selection.provenance,
                reputation_summary,
                reputation_provenance: reputation_selection.provenance,
                soul_config: config,
                adaptation_state: effective_overrides.adaptation_state.clone(),
                reader_warnings,
                generated_at: deps.now(),
            },
        )?;

        Ok(PreparedComposeInputs {
            normalized,
            effective_overrides,
        })
    }

    fn build_context(
        &self,
        deps: &AppDeps,
        normalized: NormalizedInputs,
    ) -> Result<BehavioralContext, ServiceError> {
        let compose_mode = ComposeModeService.resolve(&normalized);
        let status_summary = ComposeModeService.build_status_summary(&normalized, compose_mode);

        build_variant_context(deps, normalized, compose_mode, status_summary)
    }
}

fn build_variant_context(
    deps: &AppDeps,
    normalized: NormalizedInputs,
    compose_mode: ComposeMode,
    status_summary: StatusSummary,
) -> Result<BehavioralContext, ServiceError> {
    match compose_mode {
        ComposeMode::FailClosed => build_fail_closed_context(deps, normalized, status_summary),
        ComposeMode::Restricted => build_restricted_context(deps, normalized, status_summary),
        _ => build_rendered_context(deps, normalized, status_summary, compose_mode),
    }
}

fn build_fail_closed_context(
    deps: &AppDeps,
    normalized: NormalizedInputs,
    status_summary: StatusSummary,
) -> Result<BehavioralContext, ServiceError> {
    let profile_name = normalized.profile_name.clone();
    let prompt_prefix =
        render_prompt_prefix(deps, &normalized, ComposeMode::FailClosed, &profile_name)?;
    let fail_closed_inputs = fail_closed_inputs(&normalized);

    Ok(BehavioralContext {
        schema_version: CURRENT_SCHEMA_VERSION,
        agent_id: normalized.agent_id.clone(),
        profile_name,
        status_summary,
        baseline_trait_profile: EffectiveProfileService.derive_baseline(&fail_closed_inputs),
        trait_profile: EffectiveProfileService.derive(&fail_closed_inputs, ComposeMode::FailClosed),
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
        provenance: ProvenanceService.build(deps.provenance_hasher(), &normalized)?,
    })
}

fn build_restricted_context(
    deps: &AppDeps,
    normalized: NormalizedInputs,
    status_summary: StatusSummary,
) -> Result<BehavioralContext, ServiceError> {
    let profile_name = normalized.profile_name.clone();
    let prompt_prefix =
        render_prompt_prefix(deps, &normalized, ComposeMode::Restricted, &profile_name)?;
    let restricted_inputs = restricted_inputs(&normalized);

    Ok(BehavioralContext {
        schema_version: CURRENT_SCHEMA_VERSION,
        agent_id: normalized.agent_id.clone(),
        profile_name,
        status_summary,
        baseline_trait_profile: EffectiveProfileService.derive_baseline(&restricted_inputs),
        trait_profile: EffectiveProfileService.derive(&restricted_inputs, ComposeMode::Restricted),
        communication_rules: restricted_communication_rules(&restricted_inputs),
        decision_rules: restricted_decision_rules(&restricted_inputs),
        active_commitments: restricted_commitments(&restricted_inputs),
        relationship_context: restricted_relationships(&restricted_inputs),
        adaptive_notes: Vec::new(),
        warnings: WarningService.derive(&normalized, ComposeMode::Restricted),
        system_prompt_prefix: prompt_prefix,
        provenance: ProvenanceService.build(deps.provenance_hasher(), &normalized)?,
    })
}

fn build_rendered_context(
    deps: &AppDeps,
    normalized: NormalizedInputs,
    status_summary: StatusSummary,
    compose_mode: ComposeMode,
) -> Result<BehavioralContext, ServiceError> {
    let profile_name = normalized.profile_name.clone();
    let prompt_prefix = render_prompt_prefix(deps, &normalized, compose_mode, &profile_name)?;

    Ok(BehavioralContext {
        schema_version: CURRENT_SCHEMA_VERSION,
        agent_id: normalized.agent_id.clone(),
        profile_name,
        status_summary,
        baseline_trait_profile: EffectiveProfileService.derive_baseline(&normalized),
        trait_profile: EffectiveProfileService.derive(&normalized, compose_mode),
        communication_rules: CommunicationRulesService.derive(&normalized, compose_mode),
        decision_rules: DecisionRulesService.derive(&normalized, compose_mode),
        active_commitments: CommitmentsService.derive(&normalized, compose_mode),
        relationship_context: RelationshipsService.derive(&normalized, compose_mode),
        adaptive_notes: normalized.adaptation_state.notes.clone(),
        warnings: WarningService.derive(&normalized, compose_mode),
        system_prompt_prefix: prompt_prefix,
        provenance: ProvenanceService.build(deps.provenance_hasher(), &normalized)?,
    })
}

fn render_prompt_prefix(
    deps: &AppDeps,
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
    profile_name: &str,
) -> Result<String, ServiceError> {
    deps.render_prompt_prefix(
        &normalized.soul_config.templates.prompt_prefix_template,
        compose_mode,
        profile_name,
        normalized.soul_config.limits.max_prompt_prefix_chars,
    )
}

fn fail_closed_inputs(normalized: &NormalizedInputs) -> NormalizedInputs {
    let mut fail_closed = normalized.clone();
    fail_closed.adaptation_state = Default::default();
    fail_closed.soul_config.adaptation.enabled = false;
    fail_closed
}

fn restricted_inputs(normalized: &NormalizedInputs) -> NormalizedInputs {
    let mut restricted = normalized.clone();
    restricted.adaptation_state.notes.clear();
    restricted
}

fn restricted_communication_rules(normalized: &NormalizedInputs) -> Vec<String> {
    let mut rules = vec![
        "State the restricted mode plainly before proposing next steps.".to_owned(),
        "Keep scope narrow and avoid presenting risky follow-through as the default.".to_owned(),
    ];
    rules.extend(CommunicationRulesService.derive(normalized, ComposeMode::Restricted));
    rules
}

fn restricted_decision_rules(normalized: &NormalizedInputs) -> Vec<String> {
    let mut rules = vec![
        "Do not take risky, stateful, or autonomy-expanding actions without operator confirmation."
            .to_owned(),
        "Keep work reversible and bounded while registry standing remains suspended.".to_owned(),
    ];
    rules.extend(DecisionRulesService.derive(normalized, ComposeMode::Restricted));
    rules
}

fn restricted_commitments(normalized: &NormalizedInputs) -> Vec<String> {
    let mut commitments = vec![
        "Restricted mode is active; loaded commitments stay constrained until the operator confirms scope."
            .to_owned(),
    ];
    commitments.extend(CommitmentsService.derive(normalized, ComposeMode::Restricted));
    commitments
}

fn restricted_relationships(normalized: &NormalizedInputs) -> Vec<String> {
    let mut relationships = vec![
        "Restricted mode is active; relationship markers provide context but do not authorize autonomous escalation."
            .to_owned(),
    ];
    relationships.extend(RelationshipsService.derive(normalized, ComposeMode::Restricted));
    relationships
}

fn should_refresh_context_cache(
    identity_source: InputSourceKind,
    verification_source: InputSourceKind,
    reputation_source: InputSourceKind,
) -> bool {
    [identity_source, verification_source, reputation_source]
        .into_iter()
        .any(|source| matches!(source, InputSourceKind::Live | InputSourceKind::Explicit))
}

fn cache_identity_fingerprint(
    deps: &AppDeps,
    snapshot: Option<&SessionIdentitySnapshot>,
) -> Result<Option<String>, crate::domain::SoulError> {
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };

    match snapshot.fingerprint.clone() {
        Some(fingerprint) => Ok(Some(fingerprint)),
        None => deps
            .provenance_hasher()
            .identity_fingerprint(snapshot)
            .map(Some),
    }
}

fn invalidate_stale_cache_backed_selections(
    request: &ComposeRequest,
    config_hash: &str,
    adaptation_hash: &str,
    identity_selection: &mut ReaderSelection<IdentifySignals>,
    verification_selection: &mut ReaderSelection<VerificationResult>,
    reputation_selection: &mut ReaderSelection<ReputationSummary>,
) -> Result<(), crate::domain::SoulError> {
    if !selection_is_cache_backed(identity_selection)
        && !selection_is_cache_backed(verification_selection)
        && !selection_is_cache_backed(reputation_selection)
    {
        return Ok(());
    }

    let cached = read_cached_inputs(request)?;
    let stale_reason = cached.cached_inputs.as_ref().and_then(|cached_inputs| {
        stale_reason_against_current_inputs(cached_inputs, config_hash, adaptation_hash)
    });

    let Some(reason) = stale_reason else {
        return Ok(());
    };

    let warning = cache_stale_warning(
        &WorkspacePaths::new(&request.workspace_id).context_cache_path(),
        &reason,
    );
    invalidate_cache_selection(
        identity_selection,
        "identity snapshot unavailable",
        warning.clone(),
    );
    invalidate_cache_selection(
        verification_selection,
        "registry verification unavailable",
        warning.clone(),
    );
    invalidate_cache_selection(
        reputation_selection,
        "registry reputation unavailable",
        warning,
    );
    Ok(())
}

fn stale_reason_against_current_inputs(
    cached_inputs: &CachedInputs,
    config_hash: &str,
    adaptation_hash: &str,
) -> Option<String> {
    let freshness = cached_inputs.freshness.as_ref()?;

    if freshness.config_hash.as_deref() != Some(config_hash) {
        return Some("soul config changed".to_owned());
    }

    if freshness.adaptation_hash.as_deref() != Some(adaptation_hash) {
        return Some("adaptation state changed".to_owned());
    }

    None
}

fn selection_is_cache_backed<T>(selection: &ReaderSelection<T>) -> bool {
    matches!(selection.provenance.source, InputSourceKind::Cache)
}

fn invalidate_cache_selection<T>(
    selection: &mut ReaderSelection<T>,
    unavailable_detail: &str,
    warning: BehaviorWarning,
) {
    if !selection_is_cache_backed(selection) {
        return;
    }

    let mut unavailable =
        ReaderSelection::unavailable(InputProvenance::unavailable(unavailable_detail));
    unavailable.warnings = selection.warnings.clone();
    unavailable.warnings.push(warning);
    *selection = unavailable;
}

fn cache_write_warning(
    request: &ComposeRequest,
    error: crate::domain::SoulError,
) -> BehaviorWarning {
    BehaviorWarning {
        severity: WarningSeverity::Caution,
        code: "context_cache_write_failed".to_owned(),
        message: format!(
            "Context cache at `{}` could not be updated and was ignored: {error}",
            WorkspacePaths::new(&request.workspace_id)
                .context_cache_path()
                .display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{DateTime, TimeZone, Utc};

    use crate::{
        adaptation::EffectiveOverrideSet,
        app::config::WorkspacePaths,
        app::deps::{AdaptationStateLoader, AppDeps, ComposeClock, SoulConfigLoader},
        domain::{
            AdaptationConfig, AdaptationState, BehaviorInputs, ComposeMode, ComposeRequest,
            DecisionHeuristic, InputSourceKind, NormalizedInputs, PersonalityOverride,
            PersonalityProfile, RecoveryState, RegistryStatus, RelationshipMarker,
            ReputationSummary, SessionIdentitySnapshot, SoulConfig, SoulError, VerificationResult,
        },
        services::{provenance::ProvenanceHasher, templates::PromptTemplateRenderer},
        sources::cache::{
            CachedFreshness, CachedInputs, read_cached_inputs_path, write_cached_inputs,
        },
        sources::normalize::normalize_inputs,
        storage::sqlite::{AdaptationStateRecord, open_database, upsert_adaptation_state},
    };

    use super::ComposeService;

    #[derive(Debug, Clone)]
    struct StubConfigLoader {
        config: SoulConfig,
    }

    impl SoulConfigLoader for StubConfigLoader {
        fn load(&self, _workspace_root: &str) -> Result<SoulConfig, SoulError> {
            Ok(self.config.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct StubAdaptationLoader {
        overrides: EffectiveOverrideSet,
    }

    impl AdaptationStateLoader for StubAdaptationLoader {
        fn load_effective_overrides(
            &self,
            _workspace_root: &str,
            _config: &SoulConfig,
            _agent_id: &str,
        ) -> Result<EffectiveOverrideSet, SoulError> {
            Ok(self.overrides.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct StubClock;

    impl ComposeClock for StubClock {
        fn now(&self) -> DateTime<Utc> {
            test_timestamp(2026, 3, 29, 8, 30, 0).expect("fixed timestamp should be valid")
        }
    }

    #[derive(Debug, Clone)]
    struct StubRenderer;

    impl PromptTemplateRenderer for StubRenderer {
        fn render_prompt_prefix(
            &self,
            template_name: &str,
            compose_mode: ComposeMode,
            profile_name: &str,
            max_chars: usize,
        ) -> Result<String, SoulError> {
            Ok(format!(
                "deps:{template_name}:{compose_mode:?}:{profile_name}:{max_chars}"
            ))
        }
    }

    #[derive(Debug, Clone)]
    struct StubHasher;

    impl ProvenanceHasher for StubHasher {
        fn identity_fingerprint(
            &self,
            _snapshot: &SessionIdentitySnapshot,
        ) -> Result<String, SoulError> {
            Ok("id_deps".to_owned())
        }

        fn config_hash(&self, _config: &SoulConfig) -> Result<String, SoulError> {
            Ok("cfg_deps".to_owned())
        }

        fn adaptation_hash(&self, _state: &AdaptationState) -> Result<String, SoulError> {
            Ok("adp_deps".to_owned())
        }

        fn input_hash(&self, normalized: &NormalizedInputs) -> Result<String, SoulError> {
            Ok(format!("inp_{}", normalized.generated_at.to_rfc3339()))
        }
    }

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
            min_persist_interval_seconds: 300,
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
            &crate::app::deps::AppDeps::default(),
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

        assert_eq!(context.baseline_trait_profile.verbosity, 0.34);
        assert_eq!(context.trait_profile.verbosity, 0.49);
        assert_eq!(
            context.communication_rules,
            vec![
                "Avoid claiming identity-derived commitments or relationship context that was not loaded."
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
                "Do not infer relationship-specific obligations that are absent from the loaded baseline inputs."
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
    fn compose_uses_injected_app_deps_boundary() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-injected-deps");
        fs::create_dir_all(&workspace)?;

        let identity_path = workspace.join("explicit_identity.json");
        fs::write(
            &identity_path,
            r#"{
                "agent_id":"agent.alpha",
                "display_name":"Alpha",
                "recovery_state":"healthy",
                "active_commitments":["follow through"]
            }"#,
        )?;

        let verification_path = workspace.join("explicit_verification.json");
        fs::write(
            &verification_path,
            r#"{
                "status":"active",
                "standing_level":"good"
            }"#,
        )?;

        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.limits.max_prompt_prefix_chars = 64;

        let overrides = EffectiveOverrideSet {
            trait_profile: config.trait_baseline.clone(),
            communication_style: config.communication_style.clone(),
            decision_heuristics: config.decision_heuristics.clone(),
            adaptation_state: AdaptationState {
                notes: vec!["injected note".to_owned()],
                ..AdaptationState::default()
            },
        };

        let deps = AppDeps::default()
            .with_config_loader(StubConfigLoader {
                config: config.clone(),
            })
            .with_adaptation_loader(StubAdaptationLoader { overrides })
            .with_template_renderer(StubRenderer)
            .with_clock(StubClock)
            .with_provenance_hasher(StubHasher);

        let context = ComposeService.compose(
            &deps,
            ComposeRequest {
                workspace_id: workspace.display().to_string(),
                agent_id: "agent.alpha".to_owned(),
                session_id: "session.alpha".to_owned(),
                identity_snapshot_path: Some(identity_path.display().to_string()),
                registry_verification_path: Some(verification_path.display().to_string()),
                registry_reputation_path: None,
                include_reputation: false,
                include_relationships: true,
                include_commitments: true,
            },
        )?;

        assert_eq!(
            context.system_prompt_prefix,
            "deps:prompt-prefix:Normal:Alpha:64"
        );
        assert_eq!(context.adaptive_notes, vec!["injected note".to_owned()]);
        assert_eq!(context.provenance.config_hash, "cfg_deps");
        assert_eq!(context.provenance.adaptation_hash, "adp_deps");
        assert_eq!(
            context.provenance.identity_fingerprint.as_deref(),
            Some("id_deps")
        );
        assert_eq!(
            context.provenance.input_hash,
            "inp_2026-03-29T08:30:00+00:00"
        );
        assert_eq!(context.status_summary.compose_mode, ComposeMode::Normal);
        assert_eq!(
            context.active_commitments,
            vec!["Active commitment: follow through".to_owned()]
        );
        assert_eq!(context.provenance.registry_verification_at, None);

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

        let deps = crate::app::deps::AppDeps::default();
        let context = ComposeService
            .build_context(&deps, normalized)
            .expect("context should build");

        assert_eq!(context.status_summary.compose_mode, ComposeMode::FailClosed);
        assert!(
            context
                .system_prompt_prefix
                .starts_with("FAIL-CLOSED: identity revoked.")
        );
        assert_eq!(context.communication_rules.len(), 4);
        assert_eq!(context.decision_rules.len(), 2);
        assert!(context.active_commitments.is_empty());
        assert!(context.relationship_context.is_empty());
        assert!(context.adaptive_notes.is_empty());
        assert_eq!(
            context.baseline_trait_profile,
            PersonalityProfile::default()
        );
        assert!(context.trait_profile.initiative <= 0.05);
        assert!(context.trait_profile.verbosity <= 0.25);
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.code == "registry_revoked")
        );
    }

    #[test]
    fn suspended_input_uses_explicit_restricted_context_builder() {
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
                    status: RegistryStatus::Suspended,
                    standing_level: Some("suspended".to_owned()),
                    reason_code: Some("policy".to_owned()),
                    verified_at: Some(Utc::now()),
                }),
                adaptation_state: AdaptationState {
                    notes: vec!["adapted note".to_owned()],
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let deps = crate::app::deps::AppDeps::default();
        let context = ComposeService
            .build_context(&deps, normalized)
            .expect("context should build");

        assert_eq!(context.status_summary.compose_mode, ComposeMode::Restricted);
        assert!(
            context
                .system_prompt_prefix
                .starts_with("RESTRICTED: identity suspended.")
        );
        assert!(
            context
                .communication_rules
                .iter()
                .any(|rule| rule.contains("restricted mode plainly"))
        );
        assert!(
            context
                .decision_rules
                .iter()
                .any(|rule| rule.contains("autonomy-expanding"))
        );
        assert!(
            context
                .active_commitments
                .iter()
                .any(|item| item.contains("loaded commitments stay constrained"))
        );
        assert!(
            context
                .relationship_context
                .iter()
                .any(|item| item.contains("relationship markers provide context"))
        );
        assert!(context.adaptive_notes.is_empty());
        assert!(context.trait_profile.initiative <= 0.35);
        assert!(context.trait_profile.risk_tolerance <= 0.12);
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.code == "registry_suspended")
        );
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.code == "compose_restricted")
        );
    }

    #[test]
    fn compose_shapes_pending_commitments_and_relationships() {
        let request = ComposeRequest::new("agent.alpha", "session.alpha");
        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: SoulConfig {
                    agent_id: "agent.alpha".to_owned(),
                    profile_name: "Alpha".to_owned(),
                    ..SoulConfig::default()
                },
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
                    status: RegistryStatus::Pending,
                    standing_level: Some("pending".to_owned()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                reputation_summary: Some(ReputationSummary {
                    score_total: Some(2.5),
                    score_recent_30d: Some(2.1),
                    last_event_at: Some(Utc::now()),
                    context: vec!["recent dip".to_owned()],
                }),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let deps = crate::app::deps::AppDeps::default();
        let context = ComposeService
            .build_context(&deps, normalized)
            .expect("context should build");

        assert!(
            context
                .communication_rules
                .iter()
                .any(|rule| rule.contains("pending"))
        );
        assert!(
            context
                .decision_rules
                .iter()
                .any(|rule| rule.contains("pending standing"))
        );
        assert!(
            context
                .active_commitments
                .iter()
                .any(|item| item.contains("Pending commitment: protect the operator"))
        );
        assert!(
            context
                .relationship_context
                .iter()
                .any(|item| item.contains("Provisional relationship: operator -> trusted"))
        );
    }

    #[test]
    fn compose_requires_workspace_soul_config_even_when_cache_exists() -> Result<(), Box<dyn Error>>
    {
        let workspace = test_workspace("compose-missing-config");
        let paths = WorkspacePaths::new(&workspace);
        fs::create_dir_all(paths.state_dir())?;
        fs::write(
            paths.context_cache_path(),
            serde_json::to_vec(&CachedInputs {
                cache_key: None,
                freshness: None,
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "agent.alpha".to_owned(),
                    display_name: Some("Alpha".to_owned()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["cached commitment".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: Some("fingerprint".to_owned()),
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Active,
                    standing_level: Some("good".to_owned()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                reputation_summary: None,
            })?,
        )?;

        let error = ComposeService
            .compose(
                &crate::app::deps::AppDeps::default(),
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
            )
            .expect_err("missing soul config should fail before cached inputs are used");

        assert!(matches!(
            error,
            SoulError::ConfigRead { ref path, ref message }
            if path.ends_with("soul.toml") && message.contains("required soul config `soul.toml` is missing")
        ));

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn compose_returns_config_error_when_workspace_config_is_unreadable()
    -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-unreadable-config");
        let paths = WorkspacePaths::new(&workspace);
        fs::create_dir_all(paths.config_path())?;

        let error = ComposeService
            .compose(
                &crate::app::deps::AppDeps::default(),
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
            )
            .expect_err("directory at soul.toml path should produce a config read error");

        assert!(
            matches!(error, SoulError::ConfigRead { ref path, .. } if path.ends_with("soul.toml"))
        );

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn compose_succeeds_without_optional_cache_or_adaptation_storage() -> Result<(), Box<dyn Error>>
    {
        let workspace = test_workspace("compose-no-cache");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let paths = WorkspacePaths::new(&workspace);

        assert!(!paths.context_cache_path().exists());
        assert!(!paths.adaptation_db_path().exists());

        let context = ComposeService.compose(
            &crate::app::deps::AppDeps::default(),
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

        assert_eq!(context.profile_name, "Alpha");
        assert_eq!(
            context.status_summary.compose_mode,
            ComposeMode::BaselineOnly
        );
        assert!(!context.status_summary.identity_loaded);
        assert!(!context.status_summary.registry_verified);
        assert!(context.adaptive_notes.is_empty());

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn compose_uses_fresh_cache_when_authoritative_inputs_are_unchanged()
    -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-cache-hit");
        fs::create_dir_all(workspace.join(".soul"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let deps = crate::app::deps::AppDeps::default().with_provenance_hasher(StubHasher);
        let verified_at = test_timestamp(2026, 3, 29, 11, 0, 0)?;
        let request = ComposeRequest {
            workspace_id: workspace.display().to_string(),
            agent_id: "agent.alpha".to_owned(),
            session_id: "session.alpha".to_owned(),
            identity_snapshot_path: None,
            registry_verification_path: None,
            registry_reputation_path: None,
            include_reputation: true,
            include_relationships: true,
            include_commitments: true,
        };
        write_cached_inputs(
            &request,
            &CachedInputs {
                cache_key: None,
                freshness: Some(CachedFreshness {
                    config_hash: Some("cfg_deps".to_owned()),
                    adaptation_hash: Some("adp_deps".to_owned()),
                    identity_fingerprint: Some("id_deps".to_owned()),
                    registry_verification_at: Some(verified_at),
                }),
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "agent.alpha".to_owned(),
                    display_name: Some("Alpha".to_owned()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["cached commitment".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: Some("id_deps".to_owned()),
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Active,
                    standing_level: Some("good".to_owned()),
                    reason_code: None,
                    verified_at: Some(verified_at),
                }),
                reputation_summary: Some(ReputationSummary {
                    score_total: Some(4.7),
                    score_recent_30d: Some(4.5),
                    last_event_at: Some(verified_at),
                    context: vec!["cached reputation".to_owned()],
                }),
            },
        )?;

        let context = ComposeService.compose(&deps, request)?;

        assert_eq!(context.status_summary.compose_mode, ComposeMode::Normal);
        assert!(context.status_summary.identity_loaded);
        assert!(context.status_summary.registry_verified);
        assert_eq!(context.provenance.identity_source, InputSourceKind::Cache);
        assert_eq!(
            context.provenance.verification_source,
            InputSourceKind::Cache
        );
        assert_eq!(context.provenance.reputation_source, InputSourceKind::Cache);
        assert!(
            context
                .active_commitments
                .iter()
                .any(|commitment| commitment.contains("cached commitment"))
        );
        assert!(
            !context
                .warnings
                .iter()
                .any(|warning| warning.code == "context_cache_stale")
        );

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn compose_warns_and_falls_back_when_context_cache_is_invalid() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-cache-invalid");
        fs::create_dir_all(workspace.join(".soul"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        fs::write(workspace.join(".soul/context_cache.json"), "{not-json")?;

        let context = ComposeService.compose(
            &crate::app::deps::AppDeps::default(),
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

        assert_eq!(
            context.status_summary.compose_mode,
            ComposeMode::BaselineOnly
        );
        assert!(!context.status_summary.identity_loaded);
        assert!(!context.status_summary.registry_verified);
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.code == "context_cache_invalid")
        );

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn compose_live_registry_terminal_states_override_cached_verification()
    -> Result<(), Box<dyn Error>> {
        let cases = [
            (
                "suspended",
                ComposeMode::Restricted,
                RegistryStatus::Suspended,
            ),
            ("revoked", ComposeMode::FailClosed, RegistryStatus::Revoked),
        ];

        for (status, expected_mode, expected_registry_status) in cases {
            let workspace = test_workspace(&format!("compose-cache-live-{status}"));
            fs::create_dir_all(workspace.join(".soul"))?;
            write_soul_config(&workspace, "agent.alpha", "Alpha")?;
            fs::write(
                workspace.join("registry_verification.json"),
                format!(r#"{{"status":"{status}","standing_level":"watch"}}"#),
            )?;

            let deps = crate::app::deps::AppDeps::default().with_provenance_hasher(StubHasher);
            let request = ComposeRequest {
                workspace_id: workspace.display().to_string(),
                agent_id: "agent.alpha".to_owned(),
                session_id: "session.alpha".to_owned(),
                identity_snapshot_path: None,
                registry_verification_path: None,
                registry_reputation_path: None,
                include_reputation: true,
                include_relationships: true,
                include_commitments: true,
            };
            write_cached_inputs(
                &request,
                &CachedInputs {
                    cache_key: None,
                    freshness: Some(CachedFreshness {
                        config_hash: Some("cfg_deps".to_owned()),
                        adaptation_hash: Some("adp_deps".to_owned()),
                        identity_fingerprint: Some("id_deps".to_owned()),
                        registry_verification_at: Some(test_timestamp(2026, 3, 29, 10, 0, 0)?),
                    }),
                    identity_snapshot: Some(SessionIdentitySnapshot {
                        agent_id: "agent.alpha".to_owned(),
                        display_name: Some("Alpha".to_owned()),
                        recovery_state: RecoveryState::Healthy,
                        active_commitments: vec!["cached commitment".to_owned()],
                        durable_preferences: Vec::new(),
                        relationship_markers: Vec::new(),
                        facts: Vec::new(),
                        warnings: Vec::new(),
                        fingerprint: Some("id_deps".to_owned()),
                    }),
                    verification_result: Some(VerificationResult {
                        status: RegistryStatus::Active,
                        standing_level: Some("good".to_owned()),
                        reason_code: None,
                        verified_at: Some(test_timestamp(2026, 3, 29, 10, 0, 0)?),
                    }),
                    reputation_summary: None,
                },
            )?;

            let context = ComposeService.compose(&deps, request)?;

            assert_eq!(context.status_summary.compose_mode, expected_mode);
            assert_eq!(
                context.status_summary.registry_status,
                Some(expected_registry_status)
            );
            assert_eq!(
                context.provenance.verification_source,
                InputSourceKind::Live
            );

            cleanup_workspace(&workspace)?;
        }

        Ok(())
    }

    #[test]
    fn compose_ignores_cache_when_config_hash_is_stale() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-cache-stale-config");
        fs::create_dir_all(workspace.join(".soul"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let deps = crate::app::deps::AppDeps::default().with_provenance_hasher(StubHasher);
        let request = ComposeRequest {
            workspace_id: workspace.display().to_string(),
            agent_id: "agent.alpha".to_owned(),
            session_id: "session.alpha".to_owned(),
            identity_snapshot_path: None,
            registry_verification_path: None,
            registry_reputation_path: None,
            include_reputation: true,
            include_relationships: true,
            include_commitments: true,
        };
        write_cached_inputs(
            &request,
            &CachedInputs {
                cache_key: None,
                freshness: Some(CachedFreshness {
                    config_hash: Some("cfg_stale".to_owned()),
                    adaptation_hash: Some("adp_deps".to_owned()),
                    identity_fingerprint: Some("fingerprint-from-identify".to_owned()),
                    registry_verification_at: None,
                }),
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "agent.alpha".to_owned(),
                    display_name: Some("Alpha".to_owned()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["cached commitment".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: Some("fingerprint-from-identify".to_owned()),
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Active,
                    standing_level: Some("good".to_owned()),
                    reason_code: None,
                    verified_at: None,
                }),
                reputation_summary: None,
            },
        )?;

        let context = ComposeService.compose(&deps, request)?;

        assert_eq!(
            context.status_summary.compose_mode,
            ComposeMode::BaselineOnly
        );
        assert!(!context.status_summary.identity_loaded);
        assert!(!context.status_summary.registry_verified);
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.code == "context_cache_stale")
        );

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn compose_ignores_cache_when_adaptation_hash_is_stale() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-cache-stale-adaptation");
        fs::create_dir_all(workspace.join(".soul"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        let deps = crate::app::deps::AppDeps::default().with_provenance_hasher(StubHasher);
        let request = ComposeRequest {
            workspace_id: workspace.display().to_string(),
            agent_id: "agent.alpha".to_owned(),
            session_id: "session.alpha".to_owned(),
            identity_snapshot_path: None,
            registry_verification_path: None,
            registry_reputation_path: None,
            include_reputation: true,
            include_relationships: true,
            include_commitments: true,
        };
        write_cached_inputs(
            &request,
            &CachedInputs {
                cache_key: None,
                freshness: Some(CachedFreshness {
                    config_hash: Some("cfg_deps".to_owned()),
                    adaptation_hash: Some("adp_stale".to_owned()),
                    identity_fingerprint: Some("fingerprint-from-identify".to_owned()),
                    registry_verification_at: None,
                }),
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "agent.alpha".to_owned(),
                    display_name: Some("Alpha".to_owned()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["cached commitment".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: Some("fingerprint-from-identify".to_owned()),
                }),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Active,
                    standing_level: Some("good".to_owned()),
                    reason_code: None,
                    verified_at: None,
                }),
                reputation_summary: None,
            },
        )?;

        let context = ComposeService.compose(&deps, request)?;

        assert_eq!(
            context.status_summary.compose_mode,
            ComposeMode::BaselineOnly
        );
        assert!(!context.status_summary.identity_loaded);
        assert!(!context.status_summary.registry_verified);
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.code == "context_cache_stale")
        );

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn compose_refreshes_context_cache_when_authoritative_sources_are_loaded()
    -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("compose-cache-refresh");
        fs::create_dir_all(workspace.join(".soul"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        fs::write(
            workspace.join("session_identity_snapshot.json"),
            r#"{
                "agent_id":"agent.alpha",
                "display_name":"Alpha",
                "recovery_state":"healthy",
                "active_commitments":["cache refresh"]
            }"#,
        )?;
        fs::write(
            workspace.join("registry_verification.json"),
            r#"{
                "status":"active",
                "standing_level":"good"
            }"#,
        )?;

        let request = ComposeRequest {
            workspace_id: workspace.display().to_string(),
            agent_id: "agent.alpha".to_owned(),
            session_id: "session.alpha".to_owned(),
            identity_snapshot_path: Some(
                workspace
                    .join("session_identity_snapshot.json")
                    .display()
                    .to_string(),
            ),
            registry_verification_path: Some(
                workspace
                    .join("registry_verification.json")
                    .display()
                    .to_string(),
            ),
            registry_reputation_path: None,
            include_reputation: true,
            include_relationships: true,
            include_commitments: true,
        };

        let _ = ComposeService.compose(&crate::app::deps::AppDeps::default(), request)?;
        let cached = read_cached_inputs_path(
            crate::app::config::WorkspacePaths::new(&workspace).context_cache_path(),
        )?;
        let cached_inputs = cached.cached_inputs.expect("cache should be written");

        assert!(cached_inputs.cache_key.is_some());
        assert_eq!(
            cached_inputs
                .identity_snapshot
                .expect("identity snapshot")
                .agent_id,
            "agent.alpha"
        );
        assert_eq!(
            cached_inputs
                .verification_result
                .expect("verification")
                .standing_level
                .as_deref(),
            Some("good")
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

    fn cleanup_workspace(workspace: &Path) -> Result<(), Box<dyn Error>> {
        if workspace.exists() {
            fs::remove_dir_all(workspace)?;
        }
        Ok(())
    }

    fn write_soul_config(
        workspace: &Path,
        agent_id: &str,
        profile_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let config = SoulConfig {
            agent_id: agent_id.to_owned(),
            profile_name: profile_name.to_owned(),
            ..SoulConfig::default()
        };
        fs::write(workspace.join("soul.toml"), toml::to_string(&config)?)?;
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
