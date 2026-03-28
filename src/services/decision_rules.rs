use std::cmp::Reverse;

use crate::domain::{ComposeMode, DecisionHeuristic, NormalizedInputs};

#[derive(Debug, Clone, Default)]
pub struct DecisionRulesService;

impl DecisionRulesService {
    pub fn derive(&self, normalized: &NormalizedInputs, compose_mode: ComposeMode) -> Vec<String> {
        let mut heuristics = normalized.soul_config.decision_heuristics.clone();

        if normalized.soul_config.adaptation.enabled {
            for override_rule in &normalized.adaptation_state.heuristic_overrides {
                if let Some(existing) = heuristics
                    .iter_mut()
                    .find(|heuristic| heuristic.heuristic_id == override_rule.heuristic_id)
                {
                    existing.priority += override_rule.priority_delta;
                    if let Some(enabled) = override_rule.enabled {
                        existing.enabled = enabled;
                    }
                    if let Some(replacement) = &override_rule.replacement_instruction {
                        existing.instruction = replacement.clone();
                    }
                }
            }
        }

        heuristics
            .sort_by_key(|heuristic| (Reverse(heuristic.priority), heuristic.heuristic_id.clone()));

        let mut rules = mode_rules(compose_mode);
        rules.extend(
            heuristics
                .into_iter()
                .filter(is_enabled)
                .take(normalized.soul_config.limits.max_adaptive_rules)
                .map(|heuristic| heuristic.instruction),
        );
        rules
    }
}

fn is_enabled(heuristic: &DecisionHeuristic) -> bool {
    heuristic.enabled
}

fn mode_rules(compose_mode: ComposeMode) -> Vec<String> {
    match compose_mode {
        ComposeMode::Normal => Vec::new(),
        ComposeMode::BaselineOnly => vec![
            "Do not infer relationship-specific obligations that are absent from the loaded baseline inputs."
                .to_owned(),
        ],
        ComposeMode::Degraded => vec![
            "Prefer reversible actions and verification steps while upstream context is degraded."
                .to_owned(),
        ],
        ComposeMode::Restricted => vec![
            "Require operator confirmation before risky, stateful, or autonomy-expanding actions."
                .to_owned(),
        ],
        ComposeMode::FailClosed => vec![
            "Do not proceed with normal work; explain the fail-closed state and hand control to the operator."
                .to_owned(),
        ],
    }
}
