use serde::{Deserialize, Serialize};

use crate::domain::BehaviorInputs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComposeRequest {
    pub workspace_id: String,
    pub agent_id: String,
    pub session_id: String,
    #[serde(default = "default_true")]
    pub include_reputation: bool,
    #[serde(default = "default_true")]
    pub include_relationships: bool,
    #[serde(default = "default_true")]
    pub include_commitments: bool,
}

impl ComposeRequest {
    pub fn new(agent_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            workspace_id: ".".to_owned(),
            agent_id: agent_id.into(),
            session_id: session_id.into(),
            include_reputation: true,
            include_relationships: true,
            include_commitments: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ComposeMode {
    Normal,
    Restricted,
    Degraded,
    BaselineOnly,
    FailClosed,
}

pub type NormalizedInputs = BehaviorInputs;

const fn default_true() -> bool {
    true
}
