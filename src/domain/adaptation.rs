use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    ConflictStyle, FeedbackStyle, ParagraphBudget, QuestionStyle, RegisterStyle, UncertaintyStyle,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonalityOverride {
    #[serde(default)]
    pub openness: f32,
    #[serde(default)]
    pub conscientiousness: f32,
    #[serde(default)]
    pub initiative: f32,
    #[serde(default)]
    pub directness: f32,
    #[serde(default)]
    pub warmth: f32,
    #[serde(default)]
    pub risk_tolerance: f32,
    #[serde(default)]
    pub verbosity: f32,
    #[serde(default)]
    pub formality: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommunicationOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_register: Option<RegisterStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph_budget: Option<ParagraphBudget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_style: Option<QuestionStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty_style: Option<UncertaintyStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_style: Option<FeedbackStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_style: Option<ConflictStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeuristicOverride {
    pub heuristic_id: String,
    #[serde(default)]
    pub priority_delta: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_instruction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptationState {
    pub schema_version: u32,
    pub last_updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub trait_overrides: PersonalityOverride,
    #[serde(default)]
    pub communication_overrides: CommunicationOverride,
    #[serde(default)]
    pub heuristic_overrides: Vec<HeuristicOverride>,
    #[serde(default = "default_evidence_window_size")]
    pub evidence_window_size: u32,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl Default for AdaptationState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            last_updated_at: None,
            trait_overrides: PersonalityOverride::default(),
            communication_overrides: CommunicationOverride::default(),
            heuristic_overrides: Vec::new(),
            evidence_window_size: default_evidence_window_size(),
            notes: Vec::new(),
        }
    }
}

fn default_evidence_window_size() -> u32 {
    20
}
