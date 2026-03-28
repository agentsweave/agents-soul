use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RegisterStyle {
    Casual,
    Professional,
    #[default]
    ProfessionalDirect,
    ProfessionalWarm,
    Advisory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ParagraphBudget {
    #[default]
    Short,
    Medium,
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum QuestionStyle {
    #[default]
    SingleClarifierWhenNeeded,
    ClarifyBeforeRisk,
    QuestionFreeUnlessBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum UncertaintyStyle {
    HedgeWhenUnknown,
    #[default]
    ExplicitAndBounded,
    EscalateWhenCritical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FeedbackStyle {
    Diplomatic,
    #[default]
    Frank,
    EvidenceFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictStyle {
    DeEscalating,
    #[default]
    FirmRespectful,
    OperatorEscalation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommunicationStyle {
    #[serde(default)]
    pub default_register: RegisterStyle,
    #[serde(default)]
    pub paragraph_budget: ParagraphBudget,
    #[serde(default)]
    pub question_style: QuestionStyle,
    #[serde(default)]
    pub uncertainty_style: UncertaintyStyle,
    #[serde(default)]
    pub feedback_style: FeedbackStyle,
    #[serde(default)]
    pub conflict_style: ConflictStyle,
}
