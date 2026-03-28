use crate::domain::{BehavioralContext, ComposeRequest, SoulError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComposeService;

impl ComposeService {
    pub fn compose(&self, request: ComposeRequest) -> Result<BehavioralContext, SoulError> {
        if request.agent_id.trim().is_empty() {
            return Err(SoulError::InvalidRequest("agent_id is required"));
        }

        if request.session_id.trim().is_empty() {
            return Err(SoulError::InvalidRequest("session_id is required"));
        }

        Ok(BehavioralContext::skeleton(request))
    }
}
