use serde::{Deserialize, Serialize};

use super::{
    CommunicationStyle, ConflictStyle, DecisionHeuristic, FeedbackStyle, ParagraphBudget,
    PersonalityProfile, QuestionStyle, RegisterStyle, SoulConfig, SoulError, UncertaintyStyle,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SoulConfigPatch {
    #[serde(default)]
    pub trait_baseline: PersonalityProfilePatch,
    #[serde(default)]
    pub communication_style: CommunicationStylePatch,
    #[serde(default)]
    pub decision_heuristics: DecisionHeuristicPatch,
}

impl SoulConfigPatch {
    pub fn apply(&self, config: &SoulConfig) -> Result<SoulConfig, SoulError> {
        self.validate_patch()?;

        let mut updated = config.clone();
        self.trait_baseline.apply(&mut updated.trait_baseline);
        self.communication_style
            .apply(&mut updated.communication_style);
        self.decision_heuristics
            .apply(&mut updated.decision_heuristics)?;

        updated.finalize()
    }

    fn validate_patch(&self) -> Result<(), SoulError> {
        self.trait_baseline.validate_patch()?;
        self.decision_heuristics.validate_patch()?;
        Ok(())
    }
}

impl From<PersonalityProfilePatch> for SoulConfigPatch {
    fn from(trait_baseline: PersonalityProfilePatch) -> Self {
        Self {
            trait_baseline,
            ..Self::default()
        }
    }
}

impl From<CommunicationStylePatch> for SoulConfigPatch {
    fn from(communication_style: CommunicationStylePatch) -> Self {
        Self {
            communication_style,
            ..Self::default()
        }
    }
}

impl From<DecisionHeuristicPatch> for SoulConfigPatch {
    fn from(decision_heuristics: DecisionHeuristicPatch) -> Self {
        Self {
            decision_heuristics,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonalityProfilePatch {
    pub openness: Option<f32>,
    pub conscientiousness: Option<f32>,
    pub initiative: Option<f32>,
    pub directness: Option<f32>,
    pub warmth: Option<f32>,
    pub risk_tolerance: Option<f32>,
    pub verbosity: Option<f32>,
    pub formality: Option<f32>,
}

impl PersonalityProfilePatch {
    pub fn apply(&self, profile: &mut PersonalityProfile) {
        if let Some(value) = self.openness {
            profile.openness = value;
        }
        if let Some(value) = self.conscientiousness {
            profile.conscientiousness = value;
        }
        if let Some(value) = self.initiative {
            profile.initiative = value;
        }
        if let Some(value) = self.directness {
            profile.directness = value;
        }
        if let Some(value) = self.warmth {
            profile.warmth = value;
        }
        if let Some(value) = self.risk_tolerance {
            profile.risk_tolerance = value;
        }
        if let Some(value) = self.verbosity {
            profile.verbosity = value;
        }
        if let Some(value) = self.formality {
            profile.formality = value;
        }
    }

    fn validate_patch(&self) -> Result<(), SoulError> {
        let mut preview = PersonalityProfile::default();
        self.apply(&mut preview);
        preview.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CommunicationStylePatch {
    pub default_register: Option<RegisterStyle>,
    pub paragraph_budget: Option<ParagraphBudget>,
    pub question_style: Option<QuestionStyle>,
    pub uncertainty_style: Option<UncertaintyStyle>,
    pub feedback_style: Option<FeedbackStyle>,
    pub conflict_style: Option<ConflictStyle>,
}

impl CommunicationStylePatch {
    pub fn apply(&self, style: &mut CommunicationStyle) {
        if let Some(value) = self.default_register {
            style.default_register = value;
        }
        if let Some(value) = self.paragraph_budget {
            style.paragraph_budget = value;
        }
        if let Some(value) = self.question_style {
            style.question_style = value;
        }
        if let Some(value) = self.uncertainty_style {
            style.uncertainty_style = value;
        }
        if let Some(value) = self.feedback_style {
            style.feedback_style = value;
        }
        if let Some(value) = self.conflict_style {
            style.conflict_style = value;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DecisionHeuristicPatch {
    #[serde(default)]
    pub replace_all: Option<Vec<DecisionHeuristic>>,
    #[serde(default)]
    pub upsert: Vec<DecisionHeuristic>,
    #[serde(default)]
    pub remove: Vec<String>,
}

impl DecisionHeuristicPatch {
    pub fn apply(&self, heuristics: &mut Vec<DecisionHeuristic>) -> Result<(), SoulError> {
        let mut next = self
            .replace_all
            .clone()
            .unwrap_or_else(|| heuristics.clone());

        if !self.remove.is_empty() {
            next.retain(|heuristic| {
                !self
                    .remove
                    .iter()
                    .any(|target_id| target_id == &heuristic.heuristic_id)
            });
        }

        for heuristic in &self.upsert {
            if let Some(existing) = next
                .iter_mut()
                .find(|existing| existing.heuristic_id == heuristic.heuristic_id)
            {
                *existing = heuristic.clone();
            } else {
                next.push(heuristic.clone());
            }
        }

        canonicalize_heuristics(&mut next);
        *heuristics = next;
        Ok(())
    }

    fn validate_patch(&self) -> Result<(), SoulError> {
        if let Some(heuristics) = &self.replace_all {
            for heuristic in heuristics {
                heuristic.validate()?;
            }
        }

        for heuristic in &self.upsert {
            heuristic.validate()?;
        }

        for heuristic_id in &self.remove {
            if heuristic_id.trim().is_empty() {
                return Err(SoulError::EmptyField("decision_heuristics.remove[]"));
            }
        }

        Ok(())
    }
}

pub(crate) fn canonicalize_heuristics(heuristics: &mut [DecisionHeuristic]) {
    heuristics.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.heuristic_id.cmp(&right.heuristic_id))
    });
}
