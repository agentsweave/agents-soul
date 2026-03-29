use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    app::{
        deps::SoulDependencies,
        errors::{SoulHttpErrorResponse, SoulTransportError, map_soul_error},
    },
    domain::{ComposeRequest, DecisionHeuristicPatch, SoulConfig, SoulError},
    services::{ServiceError, explain::InspectHeuristicProjection},
};

pub fn heuristics_projection(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<InspectHeuristicProjection, ServiceError> {
    deps.inspect_report(request)
        .map(|report| report.heuristics_only())
}

pub fn update_heuristics(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    patch: DecisionHeuristicPatch,
) -> Result<SoulConfig, ServiceError> {
    deps.update_soul_config(workspace_root, &patch.into())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateHeuristicsRequest {
    pub workspace_root: String,
    #[serde(default)]
    pub patch: DecisionHeuristicPatch,
}

impl UpdateHeuristicsRequest {
    fn validate(&self) -> Result<(), SoulError> {
        if self.workspace_root.trim().is_empty() {
            return Err(SoulError::Validation(
                "workspace_root must not be empty".to_owned(),
            ));
        }
        if self.patch.replace_all.is_none()
            && self.patch.upsert.is_empty()
            && self.patch.remove.is_empty()
        {
            return Err(SoulError::Validation(
                "heuristics endpoint requires replace_all, upsert, or remove operations".to_owned(),
            ));
        }
        Ok(())
    }
}

pub fn handle_update_heuristics(
    deps: &SoulDependencies,
    request: UpdateHeuristicsRequest,
) -> Result<SoulConfig, SoulError> {
    request.validate()?;
    update_heuristics(deps, request.workspace_root, request.patch)
}

pub fn map_heuristics_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn heuristics_error_response(error: &SoulError) -> SoulHttpErrorResponse {
    map_heuristics_error(error).http_response()
}
