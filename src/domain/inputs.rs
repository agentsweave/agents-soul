use std::path::PathBuf;

use crate::domain::{AdaptationState, SoulConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeRequest {
    pub agent_id: String,
    pub session_id: String,
    pub workspace_root: PathBuf,
}

impl ComposeRequest {
    pub fn new(agent_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: session_id.into(),
            workspace_root: PathBuf::from("."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorInputs {
    pub request: ComposeRequest,
    pub config: SoulConfig,
    pub adaptation: AdaptationState,
}
