use std::{cmp::Reverse, collections::BTreeMap};

use crate::domain::{
    AdaptationState, CommunicationStyle, DecisionHeuristic, PersonalityOverride,
    PersonalityProfile, SoulConfig,
};

use super::{bounds::apply_trait_delta, store::StoredAdaptationState};

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveOverrideSet {
    pub trait_profile: PersonalityProfile,
    pub communication_style: CommunicationStyle,
    pub decision_heuristics: Vec<DecisionHeuristic>,
    pub adaptation_state: AdaptationState,
}

pub fn materialize_effective_overrides(
    config: &SoulConfig,
    stored_state: Option<&StoredAdaptationState>,
) -> EffectiveOverrideSet {
    let Some(stored_state) = stored_state else {
        return baseline_effective_overrides(config);
    };
    if !config.adaptation.enabled
        || stored_state.interaction_count < config.adaptation.min_interactions_for_adapt
    {
        return baseline_effective_overrides(config);
    }

    let mut adaptation_state = stored_state.adaptation_state.clone();
    adaptation_state.notes.sort();
    adaptation_state.notes.dedup();
    adaptation_state.trait_overrides = clamp_personality_override(
        &adaptation_state.trait_overrides,
        config.limits.max_trait_drift,
    );
    adaptation_state
        .heuristic_overrides
        .sort_by(|left, right| left.heuristic_id.cmp(&right.heuristic_id));
    adaptation_state
        .heuristic_overrides
        .dedup_by(|left, right| left.heuristic_id == right.heuristic_id);
    adaptation_state
        .heuristic_overrides
        .truncate(config.limits.max_adaptive_rules);

    EffectiveOverrideSet {
        trait_profile: apply_personality_override(
            &config.trait_baseline,
            &adaptation_state.trait_overrides,
            config.limits.max_trait_drift,
        ),
        communication_style: apply_communication_override(
            &config.communication_style,
            &adaptation_state,
        ),
        decision_heuristics: apply_heuristic_overrides(
            &config.decision_heuristics,
            &adaptation_state,
        ),
        adaptation_state,
    }
}

fn baseline_effective_overrides(config: &SoulConfig) -> EffectiveOverrideSet {
    EffectiveOverrideSet {
        trait_profile: config.trait_baseline.clone(),
        communication_style: config.communication_style.clone(),
        decision_heuristics: config.decision_heuristics.clone(),
        adaptation_state: AdaptationState::default(),
    }
}

fn clamp_personality_override(
    override_set: &PersonalityOverride,
    max_trait_drift: f32,
) -> PersonalityOverride {
    PersonalityOverride {
        openness: super::bounds::clamp_trait_delta(override_set.openness, max_trait_drift),
        conscientiousness: super::bounds::clamp_trait_delta(
            override_set.conscientiousness,
            max_trait_drift,
        ),
        initiative: super::bounds::clamp_trait_delta(override_set.initiative, max_trait_drift),
        directness: super::bounds::clamp_trait_delta(override_set.directness, max_trait_drift),
        warmth: super::bounds::clamp_trait_delta(override_set.warmth, max_trait_drift),
        risk_tolerance: super::bounds::clamp_trait_delta(
            override_set.risk_tolerance,
            max_trait_drift,
        ),
        verbosity: super::bounds::clamp_trait_delta(override_set.verbosity, max_trait_drift),
        formality: super::bounds::clamp_trait_delta(override_set.formality, max_trait_drift),
    }
}

fn apply_personality_override(
    baseline: &PersonalityProfile,
    override_set: &PersonalityOverride,
    max_trait_drift: f32,
) -> PersonalityProfile {
    PersonalityProfile {
        openness: apply_trait_delta(baseline.openness, override_set.openness, max_trait_drift),
        conscientiousness: apply_trait_delta(
            baseline.conscientiousness,
            override_set.conscientiousness,
            max_trait_drift,
        ),
        initiative: apply_trait_delta(
            baseline.initiative,
            override_set.initiative,
            max_trait_drift,
        ),
        directness: apply_trait_delta(
            baseline.directness,
            override_set.directness,
            max_trait_drift,
        ),
        warmth: apply_trait_delta(baseline.warmth, override_set.warmth, max_trait_drift),
        risk_tolerance: apply_trait_delta(
            baseline.risk_tolerance,
            override_set.risk_tolerance,
            max_trait_drift,
        ),
        verbosity: apply_trait_delta(baseline.verbosity, override_set.verbosity, max_trait_drift),
        formality: apply_trait_delta(baseline.formality, override_set.formality, max_trait_drift),
    }
}

fn apply_communication_override(
    baseline: &CommunicationStyle,
    adaptation_state: &AdaptationState,
) -> CommunicationStyle {
    CommunicationStyle {
        default_register: adaptation_state
            .communication_overrides
            .default_register
            .unwrap_or(baseline.default_register),
        paragraph_budget: adaptation_state
            .communication_overrides
            .paragraph_budget
            .unwrap_or(baseline.paragraph_budget),
        question_style: adaptation_state
            .communication_overrides
            .question_style
            .unwrap_or(baseline.question_style),
        uncertainty_style: adaptation_state
            .communication_overrides
            .uncertainty_style
            .unwrap_or(baseline.uncertainty_style),
        feedback_style: adaptation_state
            .communication_overrides
            .feedback_style
            .unwrap_or(baseline.feedback_style),
        conflict_style: adaptation_state
            .communication_overrides
            .conflict_style
            .unwrap_or(baseline.conflict_style),
    }
}

fn apply_heuristic_overrides(
    baseline: &[DecisionHeuristic],
    adaptation_state: &AdaptationState,
) -> Vec<DecisionHeuristic> {
    let overrides = adaptation_state
        .heuristic_overrides
        .iter()
        .cloned()
        .map(|override_rule| (override_rule.heuristic_id.clone(), override_rule))
        .collect::<BTreeMap<_, _>>();
    let mut heuristics = baseline.to_vec();

    for heuristic in &mut heuristics {
        if let Some(override_rule) = overrides.get(&heuristic.heuristic_id) {
            heuristic.priority += override_rule.priority_delta;
            if let Some(enabled) = override_rule.enabled {
                heuristic.enabled = enabled;
            }
            if let Some(instruction) = &override_rule.replacement_instruction {
                heuristic.instruction = instruction.clone();
            }
        }
    }

    heuristics
        .sort_by_key(|heuristic| (Reverse(heuristic.priority), heuristic.heuristic_id.clone()));
    heuristics
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        AdaptationState, CommunicationOverride, ConflictStyle, DecisionHeuristic, FeedbackStyle,
        HeuristicOverride, ParagraphBudget, PersonalityOverride, QuestionStyle, RegisterStyle,
        SoulConfig, UncertaintyStyle,
    };

    use super::{StoredAdaptationState, materialize_effective_overrides};

    #[test]
    fn materialization_clamps_trait_drift_and_limits_adaptive_rules() {
        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.limits.max_trait_drift = 0.10;
        config.limits.max_adaptive_rules = 1;
        config.adaptation.min_interactions_for_adapt = 3;
        config.decision_heuristics = vec![
            DecisionHeuristic {
                heuristic_id: "beta".to_owned(),
                title: "Beta".to_owned(),
                priority: 1,
                trigger: "review".to_owned(),
                instruction: "Use baseline beta".to_owned(),
                enabled: true,
                ..DecisionHeuristic::default()
            },
            DecisionHeuristic {
                heuristic_id: "alpha".to_owned(),
                title: "Alpha".to_owned(),
                priority: 5,
                trigger: "review".to_owned(),
                instruction: "Use baseline alpha".to_owned(),
                enabled: true,
                ..DecisionHeuristic::default()
            },
        ];

        let stored_state = StoredAdaptationState {
            agent_id: "agent.alpha".to_owned(),
            adaptation_state: AdaptationState {
                trait_overrides: PersonalityOverride {
                    verbosity: 0.40,
                    warmth: -0.25,
                    ..PersonalityOverride::default()
                },
                communication_overrides: CommunicationOverride {
                    default_register: Some(RegisterStyle::Advisory),
                    paragraph_budget: Some(ParagraphBudget::Long),
                    question_style: Some(QuestionStyle::ClarifyBeforeRisk),
                    uncertainty_style: Some(UncertaintyStyle::EscalateWhenCritical),
                    feedback_style: Some(FeedbackStyle::EvidenceFirst),
                    conflict_style: Some(ConflictStyle::OperatorEscalation),
                },
                heuristic_overrides: vec![
                    HeuristicOverride {
                        heuristic_id: "beta".to_owned(),
                        priority_delta: 10,
                        enabled: Some(false),
                        replacement_instruction: Some("Ignore beta".to_owned()),
                        note: None,
                    },
                    HeuristicOverride {
                        heuristic_id: "alpha".to_owned(),
                        priority_delta: -3,
                        enabled: Some(true),
                        replacement_instruction: Some("Use adapted alpha".to_owned()),
                        note: None,
                    },
                ],
                notes: vec!["b".to_owned(), "a".to_owned(), "a".to_owned()],
                ..AdaptationState::default()
            },
            interaction_count: 5,
            last_interaction_at: None,
            last_reset_at: None,
            updated_at: chrono::Utc::now(),
        };

        let effective = materialize_effective_overrides(&config, Some(&stored_state));

        assert_eq!(effective.trait_profile.verbosity, 0.44);
        assert_eq!(effective.trait_profile.warmth, 0.32);
        assert_eq!(
            effective.communication_style.default_register,
            RegisterStyle::Advisory
        );
        assert_eq!(
            effective.communication_style.paragraph_budget,
            ParagraphBudget::Long
        );
        assert_eq!(effective.decision_heuristics[0].heuristic_id, "alpha");
        assert_eq!(
            effective.decision_heuristics[0].instruction,
            "Use adapted alpha"
        );
        assert_eq!(effective.adaptation_state.heuristic_overrides.len(), 1);
        assert_eq!(effective.adaptation_state.notes, vec!["a", "b"]);
    }

    #[test]
    fn materialization_falls_back_to_baseline_below_threshold() {
        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.adaptation.min_interactions_for_adapt = 10;

        let stored_state = StoredAdaptationState {
            agent_id: "agent.alpha".to_owned(),
            adaptation_state: AdaptationState {
                trait_overrides: PersonalityOverride {
                    verbosity: 0.30,
                    ..PersonalityOverride::default()
                },
                ..AdaptationState::default()
            },
            interaction_count: 3,
            last_interaction_at: None,
            last_reset_at: None,
            updated_at: chrono::Utc::now(),
        };

        let effective = materialize_effective_overrides(&config, Some(&stored_state));

        assert_eq!(effective.trait_profile, config.trait_baseline);
        assert_eq!(effective.communication_style, config.communication_style);
        assert_eq!(effective.decision_heuristics, config.decision_heuristics);
        assert_eq!(effective.adaptation_state, AdaptationState::default());
    }
}
