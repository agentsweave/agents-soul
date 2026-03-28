use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptiveTrait {
    Openness,
    Conscientiousness,
    Initiative,
    Directness,
    Warmth,
    RiskTolerance,
    Verbosity,
    Formality,
}

impl AdaptiveTrait {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Openness => "openness",
            Self::Conscientiousness => "conscientiousness",
            Self::Initiative => "initiative",
            Self::Directness => "directness",
            Self::Warmth => "warmth",
            Self::RiskTolerance => "risk_tolerance",
            Self::Verbosity => "verbosity",
            Self::Formality => "formality",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InteractionOutcome {
    Positive,
    #[default]
    Neutral,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignalDirection {
    #[default]
    Increase,
    Decrease,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitSignal {
    pub trait_name: AdaptiveTrait,
    #[serde(default)]
    pub direction: SignalDirection,
    #[serde(default = "default_signal_strength")]
    pub strength: f32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeuristicPrioritySignal {
    pub heuristic_id: String,
    #[serde(default)]
    pub direction: SignalDirection,
    #[serde(default = "default_signal_strength")]
    pub strength: f32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeuristicToggleSignal {
    pub heuristic_id: String,
    pub enabled: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeuristicInstructionSignal {
    pub heuristic_id: String,
    pub instruction: String,
    #[serde(default = "default_signal_strength")]
    pub strength: f32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum InteractionSignal {
    Trait(TraitSignal),
    HeuristicPriority(HeuristicPrioritySignal),
    HeuristicToggle(HeuristicToggleSignal),
    HeuristicInstruction(HeuristicInstructionSignal),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionEvent {
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub interaction_type: String,
    #[serde(default)]
    pub outcome: InteractionOutcome,
    #[serde(default)]
    pub signals: Vec<InteractionSignal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default = "default_recorded_at")]
    pub recorded_at: DateTime<Utc>,
}

impl Default for InteractionEvent {
    fn default() -> Self {
        Self {
            agent_id: String::new(),
            session_id: None,
            interaction_type: String::new(),
            outcome: InteractionOutcome::default(),
            signals: Vec::new(),
            notes: None,
            recorded_at: default_recorded_at(),
        }
    }
}

fn default_signal_strength() -> f32 {
    1.0
}

fn default_recorded_at() -> DateTime<Utc> {
    DateTime::<Utc>::from(SystemTime::UNIX_EPOCH)
}
