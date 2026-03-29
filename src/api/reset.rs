use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    adaptation::{AdaptiveResetRequest, AdaptiveResetResult},
    app::{
        deps::SoulDependencies,
        errors::{SoulHttpErrorResponse, SoulTransportError, map_soul_error},
    },
    domain::SoulError,
    services::ServiceError,
    storage::sqlite::ResetScope,
};

pub fn reset_adaptation_state(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    request: AdaptiveResetRequest,
) -> Result<AdaptiveResetResult, ServiceError> {
    deps.reset_adaptation_state(workspace_root, &request)
}

pub fn map_reset_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn reset_error_response(error: &SoulError) -> SoulHttpErrorResponse {
    map_reset_error(error).http_response()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResetScopeRequest {
    All,
    Trait,
    Communication,
    Heuristic,
}

impl From<ResetScopeRequest> for ResetScope {
    fn from(value: ResetScopeRequest) -> Self {
        match value {
            ResetScopeRequest::All => ResetScope::All,
            ResetScopeRequest::Trait => ResetScope::Trait,
            ResetScopeRequest::Communication => ResetScope::Communication,
            ResetScopeRequest::Heuristic => ResetScope::Heuristic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetAdaptationRequest {
    pub workspace_root: String,
    pub reset_id: String,
    pub agent_id: String,
    pub scope: ResetScopeRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<DateTime<Utc>>,
}

impl ResetAdaptationRequest {
    fn validate(&self) -> Result<(), SoulError> {
        if self.workspace_root.trim().is_empty() {
            return Err(SoulError::Validation(
                "workspace_root must not be empty".to_owned(),
            ));
        }
        if self.reset_id.trim().is_empty() {
            return Err(SoulError::Validation(
                "reset_id must not be empty".to_owned(),
            ));
        }
        if self.agent_id.trim().is_empty() {
            return Err(SoulError::Validation(
                "agent_id must not be empty".to_owned(),
            ));
        }
        Ok(())
    }

    fn into_domain_request(self) -> Result<(String, AdaptiveResetRequest), SoulError> {
        self.validate()?;
        let workspace_root = self.workspace_root.clone();
        Ok((
            workspace_root,
            AdaptiveResetRequest {
                reset_id: self.reset_id,
                agent_id: self.agent_id,
                scope: self.scope.into(),
                target_key: self.target_key,
                notes: self.notes,
                recorded_at: self.recorded_at.unwrap_or_else(Utc::now),
            },
        ))
    }
}

pub fn handle_reset_adaptation(
    deps: &SoulDependencies,
    request: ResetAdaptationRequest,
) -> Result<AdaptiveResetResult, SoulError> {
    let (workspace_root, domain_request) = request.into_domain_request()?;
    reset_adaptation_state(deps, workspace_root, domain_request)
}
