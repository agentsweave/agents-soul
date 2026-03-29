use std::cmp::Reverse;

use crate::domain::{ComposeMode, DecisionHeuristic, NormalizedInputs, RegistryStatus};

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
        rules.extend(standing_rules(normalized));
        rules.extend(reputation_rules(normalized));
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

fn standing_rules(normalized: &NormalizedInputs) -> Vec<String> {
    match normalized
        .upstream
        .registry
        .verification
        .as_ref()
        .map(|verification| verification.status)
    {
        Some(RegistryStatus::Pending) => vec![
            "Treat pending standing as probationary: keep actions reversible and avoid expanding autonomy."
                .to_owned(),
        ],
        Some(RegistryStatus::Retired) => vec![
            "Treat retired standing as historical or read-only: do not start new work or accept fresh commitments."
                .to_owned(),
        ],
        _ => Vec::new(),
    }
}

fn reputation_rules(normalized: &NormalizedInputs) -> Vec<String> {
    low_reputation_score(normalized)
        .filter(|score| *score < 3.0)
        .map(|_| {
            vec![
                "Inject self-check steps, reduce confidence in unsupported claims, and prefer collaborative review because reputation is low."
                    .to_owned(),
            ]
        })
        .unwrap_or_default()
}

fn low_reputation_score(normalized: &NormalizedInputs) -> Option<f32> {
    normalized
        .upstream
        .registry
        .reputation
        .as_ref()
        .and_then(|reputation| reputation.score_recent_30d.or(reputation.score_total))
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::domain::{
        BehaviorInputs, ComposeMode, ComposeRequest, RegistryStatus, ReputationSummary, SoulConfig,
        VerificationResult,
    };
    use crate::sources::normalize::normalize_inputs;

    use super::DecisionRulesService;

    #[test]
    fn derive_adds_probationary_rule_for_pending_registry_status() {
        let normalized = normalized_inputs(
            Some(VerificationResult {
                status: RegistryStatus::Pending,
                standing_level: Some("probationary".to_owned()),
                reason_code: None,
                verified_at: Some(Utc::now()),
            }),
            None,
        );

        let rules = DecisionRulesService.derive(&normalized, ComposeMode::Normal);

        assert!(rules.iter().any(|rule| rule.contains("probationary")));
        assert!(rules.iter().any(|rule| rule.contains("reversible")));
    }

    #[test]
    fn derive_adds_historical_rule_for_retired_registry_status() {
        let normalized = normalized_inputs(
            Some(VerificationResult {
                status: RegistryStatus::Retired,
                standing_level: Some("historical".to_owned()),
                reason_code: None,
                verified_at: Some(Utc::now()),
            }),
            None,
        );

        let rules = DecisionRulesService.derive(&normalized, ComposeMode::Normal);

        assert!(rules.iter().any(|rule| rule.contains("historical")));
        assert!(rules.iter().any(|rule| rule.contains("fresh commitments")));
    }

    #[test]
    fn derive_adds_low_reputation_self_check_rule() {
        let normalized = normalized_inputs(
            Some(VerificationResult {
                status: RegistryStatus::Active,
                standing_level: Some("watch".to_owned()),
                reason_code: None,
                verified_at: Some(Utc::now()),
            }),
            Some(ReputationSummary {
                score_total: Some(2.6),
                score_recent_30d: None,
                last_event_at: None,
                context: vec!["manual review".to_owned()],
            }),
        );

        let rules = DecisionRulesService.derive(&normalized, ComposeMode::Normal);

        assert!(rules.iter().any(|rule| rule.contains("self-check steps")));
        assert!(rules.iter().any(|rule| rule.contains("reputation is low")));
    }

    fn normalized_inputs(
        verification_result: Option<VerificationResult>,
        reputation_summary: Option<ReputationSummary>,
    ) -> crate::domain::NormalizedInputs {
        let request = ComposeRequest::new("alpha", "session-1");
        let config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };

        normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                verification_result,
                reputation_summary,
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs")
    }
}
