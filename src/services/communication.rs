use crate::domain::{
    CommunicationStyle, ComposeMode, ConflictStyle, FeedbackStyle, NormalizedInputs,
    ParagraphBudget, QuestionStyle, RegisterStyle, UncertaintyStyle,
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
