use crate::domain::{
    CommunicationStyle, ComposeMode, ConflictStyle, FeedbackStyle, NormalizedInputs,
    ParagraphBudget, QuestionStyle, RegisterStyle, RegistryStatus, UncertaintyStyle,
};

#[derive(Debug, Clone, Default)]
pub struct CommunicationRulesService;

impl CommunicationRulesService {
    pub fn derive(&self, normalized: &NormalizedInputs, compose_mode: ComposeMode) -> Vec<String> {
        let mut style = normalized.soul_config.communication_style.clone();

        if normalized.soul_config.adaptation.enabled {
            let overrides = &normalized.adaptation_state.communication_overrides;
            if let Some(value) = overrides.default_register {
                style.default_register = value;
            }
            if let Some(value) = overrides.paragraph_budget {
                style.paragraph_budget = value;
            }
            if let Some(value) = overrides.question_style {
                style.question_style = value;
            }
            if let Some(value) = overrides.uncertainty_style {
                style.uncertainty_style = value;
            }
            if let Some(value) = overrides.feedback_style {
                style.feedback_style = value;
            }
            if let Some(value) = overrides.conflict_style {
                style.conflict_style = value;
            }
        }

        let mut rules = mode_rules(compose_mode);
        rules.extend(standing_rules(normalized));
        rules.extend(reputation_rules(normalized));
        rules.extend(style_rules(&style));
        rules
    }
}

fn mode_rules(compose_mode: ComposeMode) -> Vec<String> {
    match compose_mode {
        ComposeMode::Normal => Vec::new(),
        ComposeMode::BaselineOnly => vec![
            "Avoid claiming identity-derived commitments or relationship context that was not loaded."
                .to_owned(),
        ],
        ComposeMode::Degraded => vec![
            "Call out degraded or missing upstream context before acting on uncertain assumptions."
                .to_owned(),
            "Reduce autonomous initiative until identity and registry inputs are healthy again."
                .to_owned(),
        ],
        ComposeMode::Restricted => vec![
            "State the restricted mode explicitly before proposing risky or autonomous actions."
                .to_owned(),
            "Prefer operator confirmation over autonomous follow-through when scope could expand."
                .to_owned(),
        ],
        ComposeMode::FailClosed => vec![
            "Do not continue normal work; explain the fail-closed state and escalate to the operator."
                .to_owned(),
        ],
    }
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
            "Describe registry standing as pending and keep guidance probationary until activation is confirmed."
                .to_owned(),
        ],
        Some(RegistryStatus::Retired) => vec![
            "Use historical or read-only framing; do not encourage new work or fresh commitments."
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
                "Lower self-confidence, surface verification steps, and emphasize collaborative review because reputation is low."
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

fn style_rules(style: &CommunicationStyle) -> Vec<String> {
    vec![
        format!("Use a {} register.", register_text(style.default_register)),
        format!(
            "Keep responses within a {} paragraph budget.",
            paragraph_text(style.paragraph_budget)
        ),
        format!("Questions: {}.", question_text(style.question_style)),
        format!(
            "Uncertainty: {}.",
            uncertainty_text(style.uncertainty_style)
        ),
        format!("Feedback: {}.", feedback_text(style.feedback_style)),
        format!(
            "Conflict handling: {}.",
            conflict_text(style.conflict_style)
        ),
    ]
}

fn register_text(style: RegisterStyle) -> &'static str {
    match style {
        RegisterStyle::Casual => "casual",
        RegisterStyle::Professional => "professional",
        RegisterStyle::ProfessionalDirect => "professional-direct",
        RegisterStyle::ProfessionalWarm => "professional-warm",
        RegisterStyle::Advisory => "advisory",
    }
}

fn paragraph_text(style: ParagraphBudget) -> &'static str {
    match style {
        ParagraphBudget::Short => "short",
        ParagraphBudget::Medium => "medium",
        ParagraphBudget::Long => "long",
    }
}

fn question_text(style: QuestionStyle) -> &'static str {
    match style {
        QuestionStyle::SingleClarifierWhenNeeded => {
            "ask a single clarifying question only when needed"
        }
        QuestionStyle::ClarifyBeforeRisk => "clarify before risky actions",
        QuestionStyle::QuestionFreeUnlessBlocked => "avoid questions unless blocked",
    }
}

fn uncertainty_text(style: UncertaintyStyle) -> &'static str {
    match style {
        UncertaintyStyle::HedgeWhenUnknown => "hedge when facts are missing",
        UncertaintyStyle::ExplicitAndBounded => "state uncertainty explicitly and keep it bounded",
        UncertaintyStyle::EscalateWhenCritical => {
            "escalate immediately when uncertainty is critical"
        }
    }
}

fn feedback_text(style: FeedbackStyle) -> &'static str {
    match style {
        FeedbackStyle::Diplomatic => "be diplomatic",
        FeedbackStyle::Frank => "be frank",
        FeedbackStyle::EvidenceFirst => "lead with evidence",
    }
}

fn conflict_text(style: ConflictStyle) -> &'static str {
    match style {
        ConflictStyle::DeEscalating => "de-escalate where possible",
        ConflictStyle::FirmRespectful => "stay firm and respectful",
        ConflictStyle::OperatorEscalation => "escalate conflict to the operator",
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

    use super::CommunicationRulesService;

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

        let rules = CommunicationRulesService.derive(&normalized, ComposeMode::Normal);

        assert!(rules.iter().any(|rule| rule.contains("pending")));
        assert!(rules.iter().any(|rule| rule.contains("probationary")));
    }

    #[test]
    fn derive_adds_readonly_rule_for_retired_registry_status() {
        let normalized = normalized_inputs(
            Some(VerificationResult {
                status: RegistryStatus::Retired,
                standing_level: Some("historical".to_owned()),
                reason_code: None,
                verified_at: Some(Utc::now()),
            }),
            None,
        );

        let rules = CommunicationRulesService.derive(&normalized, ComposeMode::Normal);

        assert!(rules.iter().any(|rule| rule.contains("historical")));
        assert!(rules.iter().any(|rule| rule.contains("read-only")));
    }

    #[test]
    fn derive_adds_low_reputation_caution_rule() {
        let normalized = normalized_inputs(
            Some(VerificationResult {
                status: RegistryStatus::Active,
                standing_level: Some("watch".to_owned()),
                reason_code: None,
                verified_at: Some(Utc::now()),
            }),
            Some(ReputationSummary {
                score_total: Some(3.8),
                score_recent_30d: Some(2.4),
                last_event_at: None,
                context: vec!["recent incident".to_owned()],
            }),
        );

        let rules = CommunicationRulesService.derive(&normalized, ComposeMode::Normal);

        assert!(rules.iter().any(
            |rule| rule.contains("reputation is low") && rule.contains("collaborative review")
        ));
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
