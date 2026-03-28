use serde::{Deserialize, Serialize};

use super::SoulError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum HeuristicSource {
    #[default]
    SoulConfig,
    Commitment {
        commitment_id: String,
    },
    Learned {
        interaction_count: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DecisionHeuristic {
    pub heuristic_id: String,
    pub title: String,
    pub priority: i32,
    pub trigger: String,
    pub instruction: String,
    #[serde(default)]
    pub source: HeuristicSource,
    #[serde(default)]
    pub enabled: bool,
}

impl DecisionHeuristic {
    pub fn validate(&self) -> Result<(), SoulError> {
        if self.heuristic_id.trim().is_empty() {
            return Err(SoulError::EmptyField("decision_heuristics[].heuristic_id"));
        }
        if self.title.trim().is_empty() {
            return Err(SoulError::EmptyField("decision_heuristics[].title"));
        }
        if self.trigger.trim().is_empty() {
            return Err(SoulError::EmptyField("decision_heuristics[].trigger"));
        }
        if self.instruction.trim().is_empty() {
            return Err(SoulError::EmptyField("decision_heuristics[].instruction"));
        }
        Ok(())
    }
}
