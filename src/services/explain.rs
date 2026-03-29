use std::cmp::Reverse;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    adaptation::EffectiveOverrideSet,
    domain::heuristics::HeuristicSource,
    domain::{
        BehaviorWarning, BehavioralContext, CommunicationOverride, ComposeMode, HeuristicOverride,
        InputSourceKind, NormalizedInputs, PersonalityOverride, ProvenanceReport, RegistryStatus,
        StatusSummary, WarningSeverity,
    },
    services::templates::{TemplateSection, TemplateService},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExplainContributorSource {
    Baseline,
    UpstreamIdentity,
    UpstreamRegistryVerification,
    UpstreamRegistryReputation,
    Adaptation,
    ComposeMode,
    Template,
    Warning,
    Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainContributor {
    pub source: ExplainContributorSource,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainFieldContributors {
    pub field: String,
    pub contributors: Vec<ExplainContributor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectReport {
    pub schema_version: u32,
    pub agent_id: String,
    pub profile_name: String,
    pub status_summary: StatusSummary,
    pub traits: InspectTraitProjection,
    pub heuristics: InspectHeuristicProjection,
    pub adaptation: InspectAdaptationProjection,
    pub warnings: InspectWarningProjection,
    pub provenance: InspectProvenanceProjection,
    pub explain_fields: Vec<ExplainFieldContributors>,
}

impl InspectReport {
    pub fn traits_only(&self) -> InspectTraitProjection {
        self.traits.clone()
    }

    pub fn heuristics_only(&self) -> InspectHeuristicProjection {
        self.heuristics.clone()
    }

    pub fn adaptation_only(&self) -> InspectAdaptationProjection {
        self.adaptation.clone()
    }

    pub fn warnings_only(&self) -> InspectWarningProjection {
        self.warnings.clone()
    }

    pub fn provenance_only(&self) -> InspectProvenanceProjection {
        self.provenance.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectTraitProjection {
    pub entries: Vec<InspectTraitEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectTraitEntry {
    pub trait_name: String,
    pub baseline: f32,
    pub adapted: f32,
    pub effective: f32,
    pub adaptation_delta: f32,
    pub adaptation_applied: bool,
    pub mode_adjusted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectHeuristicProjection {
    pub entries: Vec<InspectHeuristicEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectHeuristicEntry {
    pub heuristic_id: String,
    pub title: String,
    pub trigger: String,
    pub source: HeuristicSource,
    pub baseline_priority: i32,
    pub effective_priority: i32,
    pub baseline_enabled: bool,
    pub effective_enabled: bool,
    pub baseline_instruction: String,
    pub effective_instruction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adaptation_note: Option<String>,
    pub modified_by_adaptation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectAdaptationProjection {
    pub enabled: bool,
    pub active: bool,
    pub evidence_window_size: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated_at: Option<DateTime<Utc>>,
    pub notes: Vec<String>,
    pub trait_overrides: Vec<InspectTraitOverride>,
    pub communication_overrides: Vec<InspectCommunicationOverride>,
    pub heuristic_overrides: Vec<InspectHeuristicOverrideEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectTraitOverride {
    pub trait_name: String,
    pub delta: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectCommunicationOverride {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectHeuristicOverrideEntry {
    pub heuristic_id: String,
    pub priority_delta: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_instruction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectWarningProjection {
    pub total: usize,
    pub severe: usize,
    pub important: usize,
    pub caution: usize,
    pub info: usize,
    pub entries: Vec<BehaviorWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InspectProvenanceProjection {
    pub report: ProvenanceReport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reputation_detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainReport {
    pub schema_version: u32,
    pub agent_id: String,
    pub profile_name: String,
    pub status_summary: StatusSummary,
    pub inspect: InspectReport,
    pub rendered: String,
}

#[derive(Debug, Clone, Default)]
pub struct ExplainService;

impl ExplainService {
    pub fn extract(
        &self,
        normalized: &NormalizedInputs,
        context: &BehavioralContext,
    ) -> Vec<ExplainFieldContributors> {
        let compose_mode = context.status_summary.compose_mode;

        vec![
            ExplainFieldContributors {
                field: "status_summary".to_owned(),
                contributors: status_summary_contributors(normalized, context),
            },
            ExplainFieldContributors {
                field: "baseline_trait_profile".to_owned(),
                contributors: baseline_trait_profile_contributors(normalized),
            },
            ExplainFieldContributors {
                field: "trait_profile".to_owned(),
                contributors: trait_profile_contributors(normalized, compose_mode),
            },
            ExplainFieldContributors {
                field: "communication_rules".to_owned(),
                contributors: communication_rule_contributors(normalized, compose_mode),
            },
            ExplainFieldContributors {
                field: "decision_rules".to_owned(),
                contributors: decision_rule_contributors(normalized, compose_mode),
            },
            ExplainFieldContributors {
                field: "active_commitments".to_owned(),
                contributors: commitment_contributors(normalized, compose_mode),
            },
            ExplainFieldContributors {
                field: "relationship_context".to_owned(),
                contributors: relationship_contributors(normalized, compose_mode),
            },
            ExplainFieldContributors {
                field: "adaptive_notes".to_owned(),
                contributors: adaptive_note_contributors(normalized, compose_mode),
            },
            ExplainFieldContributors {
                field: "warnings".to_owned(),
                contributors: warning_contributors(normalized, context),
            },
            ExplainFieldContributors {
                field: "system_prompt_prefix".to_owned(),
                contributors: prompt_prefix_contributors(normalized, compose_mode),
            },
            ExplainFieldContributors {
                field: "provenance".to_owned(),
                contributors: provenance_contributors(normalized, context),
            },
        ]
    }

    pub fn build_inspect_report(
        &self,
        normalized: &NormalizedInputs,
        effective_overrides: &EffectiveOverrideSet,
        context: &BehavioralContext,
    ) -> InspectReport {
        InspectReport {
            schema_version: context.schema_version,
            agent_id: context.agent_id.clone(),
            profile_name: context.profile_name.clone(),
            status_summary: context.status_summary.clone(),
            traits: inspect_traits(normalized, effective_overrides, context),
            heuristics: inspect_heuristics(normalized, effective_overrides),
            adaptation: inspect_adaptation(normalized),
            warnings: inspect_warnings(context),
            provenance: inspect_provenance(normalized, context),
            explain_fields: self.extract(normalized, context),
        }
    }

    pub fn build_explain_report(
        &self,
        normalized: &NormalizedInputs,
        effective_overrides: &EffectiveOverrideSet,
        context: &BehavioralContext,
    ) -> Result<ExplainReport, crate::domain::SoulError> {
        let inspect = self.build_inspect_report(normalized, effective_overrides, context);
        let rendered = TemplateService::default().render_explain(
            &normalized.soul_config.templates.explain_template,
            &format!("Explain {}", context.profile_name),
            &explain_sections(&inspect),
        )?;

        Ok(ExplainReport {
            schema_version: context.schema_version,
            agent_id: context.agent_id.clone(),
            profile_name: context.profile_name.clone(),
            status_summary: context.status_summary.clone(),
            inspect,
            rendered,
        })
    }
}

fn inspect_traits(
    normalized: &NormalizedInputs,
    effective_overrides: &EffectiveOverrideSet,
    context: &BehavioralContext,
) -> InspectTraitProjection {
    let baseline = &normalized.soul_config.trait_baseline;
    let adapted = &effective_overrides.trait_profile;
    let effective = &context.trait_profile;

    InspectTraitProjection {
        entries: vec![
            inspect_trait_entry(
                "openness",
                baseline.openness,
                adapted.openness,
                effective.openness,
            ),
            inspect_trait_entry(
                "conscientiousness",
                baseline.conscientiousness,
                adapted.conscientiousness,
                effective.conscientiousness,
            ),
            inspect_trait_entry(
                "initiative",
                baseline.initiative,
                adapted.initiative,
                effective.initiative,
            ),
            inspect_trait_entry(
                "directness",
                baseline.directness,
                adapted.directness,
                effective.directness,
            ),
            inspect_trait_entry("warmth", baseline.warmth, adapted.warmth, effective.warmth),
            inspect_trait_entry(
                "risk_tolerance",
                baseline.risk_tolerance,
                adapted.risk_tolerance,
                effective.risk_tolerance,
            ),
            inspect_trait_entry(
                "verbosity",
                baseline.verbosity,
                adapted.verbosity,
                effective.verbosity,
            ),
            inspect_trait_entry(
                "formality",
                baseline.formality,
                adapted.formality,
                effective.formality,
            ),
        ],
    }
}

fn inspect_trait_entry(
    trait_name: &str,
    baseline: f32,
    adapted: f32,
    effective: f32,
) -> InspectTraitEntry {
    let adaptation_delta = adapted - baseline;
    InspectTraitEntry {
        trait_name: trait_name.to_owned(),
        baseline,
        adapted,
        effective,
        adaptation_delta,
        adaptation_applied: adaptation_delta.abs() > f32::EPSILON,
        mode_adjusted: (effective - adapted).abs() > f32::EPSILON,
    }
}

fn inspect_heuristics(
    normalized: &NormalizedInputs,
    effective_overrides: &EffectiveOverrideSet,
) -> InspectHeuristicProjection {
    let override_notes = normalized
        .adaptation_state
        .heuristic_overrides
        .iter()
        .map(|override_rule| (override_rule.heuristic_id.as_str(), override_rule))
        .collect::<std::collections::BTreeMap<_, _>>();
    let effective_map = effective_overrides
        .decision_heuristics
        .iter()
        .map(|heuristic| (heuristic.heuristic_id.as_str(), heuristic))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut entries = normalized
        .soul_config
        .decision_heuristics
        .iter()
        .map(|baseline| {
            let effective = effective_map
                .get(baseline.heuristic_id.as_str())
                .copied()
                .unwrap_or(baseline);
            let override_rule = override_notes.get(baseline.heuristic_id.as_str()).copied();

            InspectHeuristicEntry {
                heuristic_id: baseline.heuristic_id.clone(),
                title: baseline.title.clone(),
                trigger: baseline.trigger.clone(),
                source: effective.source.clone(),
                baseline_priority: baseline.priority,
                effective_priority: effective.priority,
                baseline_enabled: baseline.enabled,
                effective_enabled: effective.enabled,
                baseline_instruction: baseline.instruction.clone(),
                effective_instruction: effective.instruction.clone(),
                adaptation_note: override_rule.and_then(|rule| rule.note.clone()),
                modified_by_adaptation: baseline.priority != effective.priority
                    || baseline.enabled != effective.enabled
                    || baseline.instruction != effective.instruction
                    || override_rule.and_then(|rule| rule.note.as_ref()).is_some(),
            }
        })
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| {
        (
            Reverse(entry.effective_priority),
            entry.heuristic_id.clone(),
        )
    });

    InspectHeuristicProjection { entries }
}

fn inspect_adaptation(normalized: &NormalizedInputs) -> InspectAdaptationProjection {
    let adaptation = &normalized.adaptation_state;
    let trait_overrides = inspect_trait_overrides(&adaptation.trait_overrides);
    let communication_overrides =
        inspect_communication_overrides(&adaptation.communication_overrides);
    let heuristic_overrides = adaptation
        .heuristic_overrides
        .iter()
        .map(|override_rule| InspectHeuristicOverrideEntry {
            heuristic_id: override_rule.heuristic_id.clone(),
            priority_delta: override_rule.priority_delta,
            enabled: override_rule.enabled,
            replacement_instruction: override_rule.replacement_instruction.clone(),
            note: override_rule.note.clone(),
        })
        .collect::<Vec<_>>();
    let active = !trait_overrides.is_empty()
        || !communication_overrides.is_empty()
        || !heuristic_overrides.is_empty()
        || !adaptation.notes.is_empty();

    InspectAdaptationProjection {
        enabled: normalized.soul_config.adaptation.enabled,
        active,
        evidence_window_size: adaptation.evidence_window_size,
        last_updated_at: adaptation.last_updated_at,
        notes: adaptation.notes.clone(),
        trait_overrides,
        communication_overrides,
        heuristic_overrides,
    }
}

fn inspect_trait_overrides(overrides: &PersonalityOverride) -> Vec<InspectTraitOverride> {
    [
        ("openness", overrides.openness),
        ("conscientiousness", overrides.conscientiousness),
        ("initiative", overrides.initiative),
        ("directness", overrides.directness),
        ("warmth", overrides.warmth),
        ("risk_tolerance", overrides.risk_tolerance),
        ("verbosity", overrides.verbosity),
        ("formality", overrides.formality),
    ]
    .into_iter()
    .filter(|(_, delta)| delta.abs() > f32::EPSILON)
    .map(|(trait_name, delta)| InspectTraitOverride {
        trait_name: trait_name.to_owned(),
        delta,
    })
    .collect()
}

fn inspect_communication_overrides(
    overrides: &CommunicationOverride,
) -> Vec<InspectCommunicationOverride> {
    let mut entries = Vec::new();

    if let Some(value) = overrides.default_register {
        entries.push(InspectCommunicationOverride {
            field: "default_register".to_owned(),
            value: format!("{value:?}"),
        });
    }
    if let Some(value) = overrides.paragraph_budget {
        entries.push(InspectCommunicationOverride {
            field: "paragraph_budget".to_owned(),
            value: format!("{value:?}"),
        });
    }
    if let Some(value) = overrides.question_style {
        entries.push(InspectCommunicationOverride {
            field: "question_style".to_owned(),
            value: format!("{value:?}"),
        });
    }
    if let Some(value) = overrides.uncertainty_style {
        entries.push(InspectCommunicationOverride {
            field: "uncertainty_style".to_owned(),
            value: format!("{value:?}"),
        });
    }
    if let Some(value) = overrides.feedback_style {
        entries.push(InspectCommunicationOverride {
            field: "feedback_style".to_owned(),
            value: format!("{value:?}"),
        });
    }
    if let Some(value) = overrides.conflict_style {
        entries.push(InspectCommunicationOverride {
            field: "conflict_style".to_owned(),
            value: format!("{value:?}"),
        });
    }

    entries
}

fn inspect_warnings(context: &BehavioralContext) -> InspectWarningProjection {
    InspectWarningProjection {
        total: context.warnings.len(),
        severe: count_warnings(&context.warnings, WarningSeverity::Severe),
        important: count_warnings(&context.warnings, WarningSeverity::Important),
        caution: count_warnings(&context.warnings, WarningSeverity::Caution),
        info: count_warnings(&context.warnings, WarningSeverity::Info),
        entries: context.warnings.clone(),
    }
}

fn inspect_provenance(
    normalized: &NormalizedInputs,
    context: &BehavioralContext,
) -> InspectProvenanceProjection {
    InspectProvenanceProjection {
        report: context.provenance.clone(),
        identity_detail: normalized.upstream.identity.provenance.detail.clone(),
        verification_detail: normalized
            .upstream
            .registry
            .verification_provenance
            .detail
            .clone(),
        reputation_detail: normalized
            .upstream
            .registry
            .reputation_provenance
            .detail
            .clone(),
    }
}

fn count_warnings(warnings: &[BehaviorWarning], severity: WarningSeverity) -> usize {
    warnings
        .iter()
        .filter(|warning| warning.severity == severity)
        .count()
}

fn explain_sections(report: &InspectReport) -> Vec<TemplateSection> {
    report
        .explain_fields
        .iter()
        .map(|field| {
            TemplateSection::new(
                explain_heading(&field.field),
                field
                    .contributors
                    .iter()
                    .map(|contributor| {
                        format!(
                            "{}: {}",
                            explain_source_label(contributor.source),
                            contributor.detail
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

fn explain_heading(field: &str) -> String {
    field
        .split('_')
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    format!("{}{}", first.to_uppercase(), chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn explain_source_label(source: ExplainContributorSource) -> &'static str {
    match source {
        ExplainContributorSource::Baseline => "Baseline",
        ExplainContributorSource::UpstreamIdentity => "Upstream identity",
        ExplainContributorSource::UpstreamRegistryVerification => "Registry verification",
        ExplainContributorSource::UpstreamRegistryReputation => "Registry reputation",
        ExplainContributorSource::Adaptation => "Adaptation",
        ExplainContributorSource::ComposeMode => "Compose mode",
        ExplainContributorSource::Template => "Template",
        ExplainContributorSource::Warning => "Warning",
        ExplainContributorSource::Provenance => "Provenance",
    }
}

fn status_summary_contributors(
    normalized: &NormalizedInputs,
    context: &BehavioralContext,
) -> Vec<ExplainContributor> {
    let mut contributors = vec![contributor(
        ExplainContributorSource::ComposeMode,
        format!(
            "compose mode resolved to {} from registry status {:?}, recovery state {:?}, and offline policy {:?}",
            compose_mode_label(context.status_summary.compose_mode),
            normalized
                .upstream
                .registry
                .verification
                .as_ref()
                .map(|verification| verification.status),
            normalized.upstream.identity.recovery_state,
            normalized.soul_config.limits.offline_registry_behavior,
        ),
    )];

    contributors.push(contributor(
        ExplainContributorSource::UpstreamIdentity,
        format!(
            "identity_loaded={} via {}",
            context.status_summary.identity_loaded,
            source_label(normalized.upstream.identity.provenance.source)
        ),
    ));
    contributors.push(contributor(
        ExplainContributorSource::UpstreamRegistryVerification,
        format!(
            "registry_verified={} via {}",
            context.status_summary.registry_verified,
            source_label(normalized.upstream.registry.verification_provenance.source)
        ),
    ));
    contributors.push(contributor(
        ExplainContributorSource::UpstreamRegistryReputation,
        format!(
            "reputation_loaded={} via {}",
            context.status_summary.reputation_loaded,
            source_label(normalized.upstream.registry.reputation_provenance.source)
        ),
    ));

    contributors
}

fn baseline_trait_profile_contributors(_normalized: &NormalizedInputs) -> Vec<ExplainContributor> {
    vec![contributor(
        ExplainContributorSource::Baseline,
        "baseline trait profile comes directly from soul.toml trait_baseline".to_owned(),
    )]
}

fn trait_profile_contributors(
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> Vec<ExplainContributor> {
    let mut contributors = baseline_trait_profile_contributors(normalized);

    if normalized.soul_config.adaptation.enabled {
        let override_details =
            personality_override_details(&normalized.adaptation_state.trait_overrides);
        if !override_details.is_empty() {
            contributors.push(contributor(
                ExplainContributorSource::Adaptation,
                format!(
                    "trait drift overrides applied: {}",
                    override_details.join(", ")
                ),
            ));
        }
    }

    if !matches!(
        compose_mode,
        ComposeMode::Normal | ComposeMode::BaselineOnly
    ) {
        contributors.push(contributor(
            ExplainContributorSource::ComposeMode,
            format!(
                "{} mode clamps initiative/risk/formality-related traits to reduce autonomy",
                compose_mode_label(compose_mode)
            ),
        ));
    }

    contributors
}

fn communication_rule_contributors(
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> Vec<ExplainContributor> {
    let mut contributors = vec![contributor(
        ExplainContributorSource::Baseline,
        "communication rules start from soul.toml communication_style defaults".to_owned(),
    )];

    let override_details =
        communication_override_details(&normalized.adaptation_state.communication_overrides);
    if normalized.soul_config.adaptation.enabled && !override_details.is_empty() {
        contributors.push(contributor(
            ExplainContributorSource::Adaptation,
            format!(
                "communication overrides applied: {}",
                override_details.join(", ")
            ),
        ));
    }

    if !matches!(compose_mode, ComposeMode::Normal) {
        contributors.push(contributor(
            ExplainContributorSource::ComposeMode,
            format!(
                "{} mode injects explicit communication guardrails",
                compose_mode_label(compose_mode)
            ),
        ));
    }

    if let Some(status) = normalized
        .upstream
        .registry
        .verification
        .as_ref()
        .map(|verification| verification.status)
        && matches!(status, RegistryStatus::Pending | RegistryStatus::Retired)
    {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryVerification,
            format!(
                "registry status {} adds communication framing",
                registry_status_label(status)
            ),
        ));
    }

    if low_reputation_score(normalized).is_some_and(|score| score < 3.0) {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryReputation,
            "low reputation adds cautious communication guidance".to_owned(),
        ));
    }

    contributors
}

fn decision_rule_contributors(
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> Vec<ExplainContributor> {
    let mut contributors = vec![contributor(
        ExplainContributorSource::Baseline,
        format!(
            "decision rules start from {} configured heuristics",
            normalized.soul_config.decision_heuristics.len()
        ),
    )];

    let heuristic_details =
        heuristic_override_details(&normalized.adaptation_state.heuristic_overrides);
    if normalized.soul_config.adaptation.enabled && !heuristic_details.is_empty() {
        contributors.push(contributor(
            ExplainContributorSource::Adaptation,
            format!(
                "heuristic overrides applied: {}",
                heuristic_details.join(", ")
            ),
        ));
    }

    if !matches!(compose_mode, ComposeMode::Normal) {
        contributors.push(contributor(
            ExplainContributorSource::ComposeMode,
            format!(
                "{} mode prepends decision guardrails before configured heuristics",
                compose_mode_label(compose_mode)
            ),
        ));
    }

    if let Some(status) = normalized
        .upstream
        .registry
        .verification
        .as_ref()
        .map(|verification| verification.status)
        && matches!(status, RegistryStatus::Pending | RegistryStatus::Retired)
    {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryVerification,
            format!(
                "registry status {} injects decision constraints",
                registry_status_label(status)
            ),
        ));
    }

    if low_reputation_score(normalized).is_some_and(|score| score < 3.0) {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryReputation,
            "low reputation injects self-check and collaborative-review rules".to_owned(),
        ));
    }

    contributors
}

fn commitment_contributors(
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> Vec<ExplainContributor> {
    let mut contributors = Vec::new();

    if normalized.request.include_commitments {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamIdentity,
            format!(
                "active commitments come from identity snapshot via {}",
                source_label(normalized.upstream.identity.provenance.source)
            ),
        ));
    }

    if !matches!(
        compose_mode,
        ComposeMode::Normal | ComposeMode::BaselineOnly
    ) {
        contributors.push(contributor(
            ExplainContributorSource::ComposeMode,
            format!(
                "{} mode constrains how commitments are framed",
                compose_mode_label(compose_mode)
            ),
        ));
    }

    if let Some(status) = normalized
        .upstream
        .registry
        .verification
        .as_ref()
        .map(|verification| verification.status)
        && matches!(status, RegistryStatus::Pending | RegistryStatus::Retired)
    {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryVerification,
            format!(
                "registry status {} changes commitment framing",
                registry_status_label(status)
            ),
        ));
    }

    if low_reputation_score(normalized).is_some_and(|score| score < 3.0) {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryReputation,
            "low reputation adds commitment verification guidance".to_owned(),
        ));
    }

    contributors
}

fn relationship_contributors(
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> Vec<ExplainContributor> {
    let mut contributors = Vec::new();

    if normalized.request.include_relationships {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamIdentity,
            format!(
                "relationship markers come from identity snapshot via {}",
                source_label(normalized.upstream.identity.provenance.source)
            ),
        ));
    }

    if matches!(compose_mode, ComposeMode::Restricted) {
        contributors.push(contributor(
            ExplainContributorSource::ComposeMode,
            "restricted mode keeps relationship markers from bypassing approval requirements"
                .to_owned(),
        ));
    }

    if let Some(status) = normalized
        .upstream
        .registry
        .verification
        .as_ref()
        .map(|verification| verification.status)
        && matches!(status, RegistryStatus::Pending | RegistryStatus::Retired)
    {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryVerification,
            format!(
                "registry status {} changes relationship framing",
                registry_status_label(status)
            ),
        ));
    }

    if low_reputation_score(normalized).is_some_and(|score| score < 3.0) {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamRegistryReputation,
            "low reputation prevents relationship markers from substituting for verification"
                .to_owned(),
        ));
    }

    contributors
}

fn adaptive_note_contributors(
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> Vec<ExplainContributor> {
    let mut contributors = Vec::new();

    if normalized.soul_config.adaptation.enabled && !normalized.adaptation_state.notes.is_empty() {
        contributors.push(contributor(
            ExplainContributorSource::Adaptation,
            format!(
                "adaptive notes originate from {} learned notes",
                normalized.adaptation_state.notes.len()
            ),
        ));
    }

    if matches!(
        compose_mode,
        ComposeMode::Restricted | ComposeMode::FailClosed
    ) && !normalized.adaptation_state.notes.is_empty()
    {
        contributors.push(contributor(
            ExplainContributorSource::ComposeMode,
            format!(
                "{} mode suppresses adaptation notes in the final context",
                compose_mode_label(compose_mode)
            ),
        ));
    }

    contributors
}

fn warning_contributors(
    normalized: &NormalizedInputs,
    context: &BehavioralContext,
) -> Vec<ExplainContributor> {
    let mut contributors = Vec::new();

    if !normalized.reader_warnings.is_empty() {
        contributors.push(contributor(
            ExplainContributorSource::Warning,
            format!(
                "{} reader warnings fed into warning derivation",
                normalized.reader_warnings.len()
            ),
        ));
    }

    let snapshot_warning_count = normalized
        .upstream
        .identity
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.warnings.len())
        .unwrap_or_default();
    if snapshot_warning_count > 0 {
        contributors.push(contributor(
            ExplainContributorSource::UpstreamIdentity,
            format!("identity snapshot contributed {snapshot_warning_count} warnings"),
        ));
    }

    contributors.push(contributor(
        ExplainContributorSource::ComposeMode,
        format!(
            "{} warnings emitted after compose-mode-specific severity ordering and deduplication",
            context.warnings.len()
        ),
    ));

    let severe_count = context
        .warnings
        .iter()
        .filter(|warning| warning.severity == WarningSeverity::Severe)
        .count();
    if severe_count > 0 {
        contributors.push(contributor(
            ExplainContributorSource::Warning,
            format!("warning set includes {severe_count} severe warnings"),
        ));
    }

    contributors
}

fn prompt_prefix_contributors(
    normalized: &NormalizedInputs,
    compose_mode: ComposeMode,
) -> Vec<ExplainContributor> {
    vec![
        contributor(
            ExplainContributorSource::Template,
            format!(
                "prompt prefix rendered from template `{}`",
                normalized.soul_config.templates.prompt_prefix_template
            ),
        ),
        contributor(
            ExplainContributorSource::ComposeMode,
            format!(
                "render uses {} mode and profile `{}`",
                compose_mode_label(compose_mode),
                normalized.profile_name
            ),
        ),
    ]
}

fn provenance_contributors(
    normalized: &NormalizedInputs,
    context: &BehavioralContext,
) -> Vec<ExplainContributor> {
    vec![
        contributor(
            ExplainContributorSource::Provenance,
            format!(
                "config hash {} and adaptation hash {} summarize local soul inputs",
                context.provenance.config_hash, context.provenance.adaptation_hash
            ),
        ),
        contributor(
            ExplainContributorSource::UpstreamIdentity,
            format!(
                "identity provenance source is {}",
                source_label(normalized.upstream.identity.provenance.source)
            ),
        ),
        contributor(
            ExplainContributorSource::UpstreamRegistryVerification,
            format!(
                "registry verification provenance source is {}",
                source_label(normalized.upstream.registry.verification_provenance.source)
            ),
        ),
        contributor(
            ExplainContributorSource::UpstreamRegistryReputation,
            format!(
                "registry reputation provenance source is {}",
                source_label(normalized.upstream.registry.reputation_provenance.source)
            ),
        ),
        contributor(
            ExplainContributorSource::Provenance,
            format!(
                "input hash {} locks the normalized compose inputs",
                context.provenance.input_hash
            ),
        ),
    ]
}

fn personality_override_details(overrides: &PersonalityOverride) -> Vec<String> {
    [
        ("openness", overrides.openness),
        ("conscientiousness", overrides.conscientiousness),
        ("initiative", overrides.initiative),
        ("directness", overrides.directness),
        ("warmth", overrides.warmth),
        ("risk_tolerance", overrides.risk_tolerance),
        ("verbosity", overrides.verbosity),
        ("formality", overrides.formality),
    ]
    .into_iter()
    .filter(|(_, value)| *value != 0.0)
    .map(|(field, value)| format!("{field}={value:+.2}"))
    .collect()
}

fn communication_override_details(overrides: &CommunicationOverride) -> Vec<String> {
    let mut details = Vec::new();

    if let Some(value) = overrides.default_register {
        details.push(format!("default_register={value:?}"));
    }
    if let Some(value) = overrides.paragraph_budget {
        details.push(format!("paragraph_budget={value:?}"));
    }
    if let Some(value) = overrides.question_style {
        details.push(format!("question_style={value:?}"));
    }
    if let Some(value) = overrides.uncertainty_style {
        details.push(format!("uncertainty_style={value:?}"));
    }
    if let Some(value) = overrides.feedback_style {
        details.push(format!("feedback_style={value:?}"));
    }
    if let Some(value) = overrides.conflict_style {
        details.push(format!("conflict_style={value:?}"));
    }

    details
}

fn heuristic_override_details(overrides: &[HeuristicOverride]) -> Vec<String> {
    overrides
        .iter()
        .map(|override_rule| {
            let mut details = vec![override_rule.heuristic_id.clone()];
            if override_rule.priority_delta != 0 {
                details.push(format!("priority_delta={:+}", override_rule.priority_delta));
            }
            if let Some(enabled) = override_rule.enabled {
                details.push(format!("enabled={enabled}"));
            }
            if override_rule.replacement_instruction.is_some() {
                details.push("instruction=replaced".to_owned());
            }
            if override_rule.note.is_some() {
                details.push("note=present".to_owned());
            }
            details.join(" ")
        })
        .collect()
}

fn low_reputation_score(normalized: &NormalizedInputs) -> Option<f32> {
    normalized
        .upstream
        .registry
        .reputation
        .as_ref()
        .and_then(|reputation| reputation.score_recent_30d.or(reputation.score_total))
}

fn contributor(source: ExplainContributorSource, detail: String) -> ExplainContributor {
    ExplainContributor { source, detail }
}

fn compose_mode_label(mode: ComposeMode) -> &'static str {
    match mode {
        ComposeMode::Normal => "normal",
        ComposeMode::Restricted => "restricted",
        ComposeMode::Degraded => "degraded",
        ComposeMode::BaselineOnly => "baseline-only",
        ComposeMode::FailClosed => "fail-closed",
    }
}

fn source_label(source: InputSourceKind) -> &'static str {
    match source {
        InputSourceKind::Explicit => "explicit",
        InputSourceKind::Live => "live",
        InputSourceKind::Cache => "cache",
        InputSourceKind::Unavailable => "unavailable",
    }
}

fn registry_status_label(status: RegistryStatus) -> &'static str {
    match status {
        RegistryStatus::Active => "active",
        RegistryStatus::Pending => "pending",
        RegistryStatus::Suspended => "suspended",
        RegistryStatus::Revoked => "revoked",
        RegistryStatus::Retired => "retired",
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        adaptation::{EffectiveOverrideSet, materialize_effective_overrides},
        domain::{
            AdaptationState, BehaviorInputs, BehavioralContext, CommunicationOverride, ComposeMode,
            ComposeRequest, DecisionHeuristic, HeuristicOverride, InputProvenance, InputSourceKind,
            PersonalityOverride, RecoveryState, RegistryStatus, RelationshipMarker,
            ReputationSummary, SessionIdentitySnapshot, SoulConfig, VerificationResult,
        },
        services::{
            commitments::CommitmentsService,
            communication::CommunicationRulesService,
            decision_rules::DecisionRulesService,
            profile::EffectiveProfileService,
            provenance::{ProvenanceService, StableProvenanceHasher},
            relationships::RelationshipsService,
            warnings::WarningService,
        },
        sources::normalize::normalize_inputs,
    };

    use super::{ExplainContributorSource, ExplainService};

    #[test]
    fn extract_enumerates_baseline_upstream_and_adaptation_contributors() {
        let request = ComposeRequest::new("alpha", "session-1");
        let mut config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        config.decision_heuristics = vec![DecisionHeuristic {
            heuristic_id: "verify-first".into(),
            title: "Verify first".into(),
            priority: 1,
            trigger: "default".into(),
            instruction: "Verify before acting".into(),
            enabled: true,
            ..DecisionHeuristic::default()
        }];

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "alpha".into(),
                    display_name: Some("Alpha".into()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["protect operator trust".into()],
                    durable_preferences: Vec::new(),
                    relationship_markers: vec![RelationshipMarker {
                        subject: "operator".into(),
                        marker: "trusted".into(),
                        note: None,
                    }],
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: Some("id-1".into()),
                }),
                identity_provenance: InputProvenance::live("identity.json"),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Pending,
                    standing_level: Some("probationary".into()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                verification_provenance: InputProvenance::explicit("verification.json"),
                reputation_summary: Some(ReputationSummary {
                    score_total: Some(2.9),
                    score_recent_30d: Some(2.4),
                    last_event_at: None,
                    context: vec!["recent review".into()],
                }),
                reputation_provenance: InputProvenance::cache("context_cache.json"),
                adaptation_state: AdaptationState {
                    trait_overrides: PersonalityOverride {
                        initiative: -0.30,
                        directness: -0.10,
                        ..PersonalityOverride::default()
                    },
                    communication_overrides: CommunicationOverride {
                        question_style: Some(crate::domain::QuestionStyle::ClarifyBeforeRisk),
                        ..CommunicationOverride::default()
                    },
                    heuristic_overrides: vec![HeuristicOverride {
                        heuristic_id: "verify-first".into(),
                        priority_delta: 3,
                        enabled: Some(true),
                        replacement_instruction: Some("Double-check before acting".into()),
                        note: Some("learned caution".into()),
                    }],
                    notes: vec!["Recent corrections reduced initiative.".into()],
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let context = build_context(&normalized, ComposeMode::Restricted);
        let extracted = ExplainService.extract(&normalized, &context);

        let trait_profile = extracted
            .iter()
            .find(|field| field.field == "trait_profile")
            .expect("trait profile contributors");
        assert!(
            trait_profile
                .contributors
                .iter()
                .any(|item| item.source == ExplainContributorSource::Baseline)
        );
        assert!(
            trait_profile
                .contributors
                .iter()
                .any(|item| item.source == ExplainContributorSource::Adaptation)
        );
        assert!(
            trait_profile
                .contributors
                .iter()
                .any(|item| item.source == ExplainContributorSource::ComposeMode)
        );

        let communication = extracted
            .iter()
            .find(|field| field.field == "communication_rules")
            .expect("communication contributors");
        assert!(
            communication
                .contributors
                .iter()
                .any(|item| item.source == ExplainContributorSource::UpstreamRegistryVerification)
        );
        assert!(
            communication
                .contributors
                .iter()
                .any(|item| item.source == ExplainContributorSource::UpstreamRegistryReputation)
        );
        assert!(
            communication
                .contributors
                .iter()
                .any(|item| item.source == ExplainContributorSource::Adaptation)
        );

        let commitments = extracted
            .iter()
            .find(|field| field.field == "active_commitments")
            .expect("commitment contributors");
        assert!(
            commitments
                .contributors
                .iter()
                .any(|item| item.source == ExplainContributorSource::UpstreamIdentity)
        );
    }

    #[test]
    fn extract_is_deterministic_for_snapshot_use() {
        let request = ComposeRequest::new("alpha", "session-1");
        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: SoulConfig {
                    agent_id: "alpha".into(),
                    profile_name: "Alpha".into(),
                    ..SoulConfig::default()
                },
                identity_provenance: InputProvenance {
                    source: InputSourceKind::Unavailable,
                    detail: Some("identity not requested".into()),
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let context = build_context(&normalized, ComposeMode::BaselineOnly);
        let first = ExplainService.extract(&normalized, &context);
        let second = ExplainService.extract(&normalized, &context);

        assert_eq!(first, second);
    }

    #[test]
    fn build_inspect_report_exposes_structured_trait_and_warning_slices() {
        let request = ComposeRequest::new("alpha", "session-1");
        let mut config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        config.limits.max_trait_drift = 0.10;
        config.decision_heuristics = vec![DecisionHeuristic {
            heuristic_id: "verify-first".into(),
            title: "Verify first".into(),
            priority: 1,
            trigger: "default".into(),
            instruction: "Verify before acting".into(),
            enabled: true,
            ..DecisionHeuristic::default()
        }];

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config.clone(),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Suspended,
                    standing_level: Some("watch".into()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                adaptation_state: AdaptationState {
                    trait_overrides: PersonalityOverride {
                        initiative: -0.30,
                        ..PersonalityOverride::default()
                    },
                    notes: vec!["Slow down autonomy".into()],
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let effective_overrides = materialize_effective_overrides(&config, None);
        let context = build_context(&normalized, ComposeMode::Restricted);
        let report =
            ExplainService.build_inspect_report(&normalized, &effective_overrides, &context);

        assert_eq!(report.status_summary.compose_mode, ComposeMode::Restricted);
        assert!(
            report
                .traits
                .entries
                .iter()
                .any(|entry| entry.trait_name == "initiative" && entry.mode_adjusted)
        );
        assert_eq!(report.warnings.total, context.warnings.len());
        assert_eq!(report.traits_only(), report.traits);
        assert_eq!(report.warnings_only(), report.warnings);
        assert!(
            report
                .explain_fields
                .iter()
                .any(|field| field.field == "provenance")
        );
    }

    #[test]
    fn build_inspect_report_exposes_heuristic_and_adaptation_overrides() {
        let request = ComposeRequest::new("alpha", "session-1");
        let mut config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        config.adaptation.min_interactions_for_adapt = 1;
        config.decision_heuristics = vec![DecisionHeuristic {
            heuristic_id: "verify-first".into(),
            title: "Verify first".into(),
            priority: 1,
            trigger: "default".into(),
            instruction: "Verify before acting".into(),
            enabled: true,
            ..DecisionHeuristic::default()
        }];

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config.clone(),
                adaptation_state: AdaptationState {
                    communication_overrides: CommunicationOverride {
                        question_style: Some(crate::domain::QuestionStyle::ClarifyBeforeRisk),
                        ..CommunicationOverride::default()
                    },
                    heuristic_overrides: vec![HeuristicOverride {
                        heuristic_id: "verify-first".into(),
                        priority_delta: 3,
                        enabled: Some(false),
                        replacement_instruction: Some("Escalate before acting".into()),
                        note: Some("operator requested more caution".into()),
                    }],
                    notes: vec!["Recent operator corrections".into()],
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let effective_overrides = EffectiveOverrideSet {
            trait_profile: config.trait_baseline.clone(),
            communication_style: config.communication_style.clone(),
            decision_heuristics: vec![DecisionHeuristic {
                heuristic_id: "verify-first".into(),
                title: "Verify first".into(),
                priority: 4,
                trigger: "default".into(),
                instruction: "Escalate before acting".into(),
                enabled: false,
                ..DecisionHeuristic::default()
            }],
            adaptation_state: normalized.adaptation_state.clone(),
        };
        let context = build_context(&normalized, ComposeMode::Normal);
        let report =
            ExplainService.build_inspect_report(&normalized, &effective_overrides, &context);

        let heuristic = report
            .heuristics
            .entries
            .iter()
            .find(|entry| entry.heuristic_id == "verify-first")
            .expect("heuristic projection");
        assert!(heuristic.modified_by_adaptation);
        assert_eq!(heuristic.effective_priority, 4);
        assert_eq!(heuristic.effective_instruction, "Escalate before acting");
        assert_eq!(
            heuristic.adaptation_note.as_deref(),
            Some("operator requested more caution")
        );

        assert!(report.adaptation.active);
        assert_eq!(report.heuristics_only(), report.heuristics);
        assert!(
            report
                .adaptation
                .communication_overrides
                .iter()
                .any(|entry| entry.field == "question_style")
        );
    }

    #[test]
    fn build_explain_report_renders_contributor_sections() {
        let request = ComposeRequest::new("alpha", "session-1");
        let config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config.clone(),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Pending,
                    standing_level: Some("watch".into()),
                    reason_code: None,
                    verified_at: Some(Utc::now()),
                }),
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let effective_overrides = materialize_effective_overrides(&config, None);
        let context = build_context(&normalized, ComposeMode::BaselineOnly);
        let report = ExplainService
            .build_explain_report(&normalized, &effective_overrides, &context)
            .expect("explain report");

        assert!(report.rendered.contains("## Status Summary"));
        assert!(report.rendered.contains("Compose mode:"));
        assert!(report.rendered.contains("## Provenance"));
        assert_eq!(report.inspect.profile_name, "Alpha");
    }

    #[test]
    fn build_explain_report_reuses_inspect_projection() {
        let request = ComposeRequest::new("alpha", "session-1");
        let config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config.clone(),
                adaptation_state: AdaptationState {
                    notes: vec!["Operator feedback reduced initiative".into()],
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let effective_overrides = materialize_effective_overrides(&config, None);
        let context = build_context(&normalized, ComposeMode::Normal);
        let report = ExplainService
            .build_explain_report(&normalized, &effective_overrides, &context)
            .expect("explain report");

        assert_eq!(report.status_summary, report.inspect.status_summary);
        assert_eq!(
            report.inspect.explain_fields,
            ExplainService.extract(&normalized, &context)
        );
    }

    fn build_context(
        normalized: &crate::domain::NormalizedInputs,
        compose_mode: ComposeMode,
    ) -> BehavioralContext {
        BehavioralContext {
            schema_version: normalized.schema_version,
            agent_id: normalized.agent_id.clone(),
            profile_name: normalized.profile_name.clone(),
            status_summary: crate::services::limits::ComposeModeService
                .build_status_summary(normalized, compose_mode),
            baseline_trait_profile: EffectiveProfileService.derive_baseline(normalized),
            trait_profile: EffectiveProfileService.derive(normalized, compose_mode),
            communication_rules: CommunicationRulesService.derive(normalized, compose_mode),
            decision_rules: DecisionRulesService.derive(normalized, compose_mode),
            active_commitments: CommitmentsService.derive(normalized, compose_mode),
            relationship_context: RelationshipsService.derive(normalized, compose_mode),
            adaptive_notes: normalized.adaptation_state.notes.clone(),
            warnings: WarningService.derive(normalized, compose_mode),
            system_prompt_prefix: format!("prefix:{compose_mode:?}:{}", normalized.profile_name),
            provenance: ProvenanceService
                .build(&StableProvenanceHasher, normalized)
                .expect("provenance"),
        }
    }
}
