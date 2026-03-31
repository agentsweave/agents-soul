use crate::{
    adaptation::{
        AdaptiveResetRequest, AdaptiveResetResult, InteractionRecordRequest,
        InteractionRecordResult,
    },
    app::{
        deps::SoulDependencies,
        errors::{SoulMcpToolError, SoulTransportError, map_soul_error},
    },
    domain::{
        BehavioralContext, ComposeRequest, PersonalityProfilePatch, SoulConfig, SoulConfigPatch,
        SoulError,
    },
    services::{
        ServiceError,
        explain::{ExplainReport, InspectHeuristicProjection, InspectTraitProjection},
    },
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn compose_context(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<BehavioralContext, ServiceError> {
    deps.compose_context(request)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrefixToolResponse {
    pub system_prompt_prefix: String,
}

pub fn get_prefix(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<PrefixToolResponse, ServiceError> {
    deps.compose_context(request)
        .map(|context| PrefixToolResponse {
            system_prompt_prefix: context.system_prompt_prefix,
        })
}

pub fn explain_report(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<ExplainReport, ServiceError> {
    deps.explain_report(request)
}

pub fn get_traits(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<InspectTraitProjection, ServiceError> {
    deps.inspect_report(request)
        .map(|report| report.traits_only())
}

pub fn get_heuristics(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<InspectHeuristicProjection, ServiceError> {
    deps.inspect_report(request)
        .map(|report| report.heuristics_only())
}

pub fn map_compose_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn tool_error(error: &SoulError) -> SoulMcpToolError {
    map_compose_error(error).mcp_tool_error()
}

pub fn compose_tool_error(error: &SoulError) -> SoulMcpToolError {
    tool_error(error)
}

pub fn configure_workspace(
    deps: &SoulDependencies,
    workspace_root: impl Into<std::path::PathBuf>,
    patch: SoulConfigPatch,
) -> Result<SoulConfig, ServiceError> {
    deps.update_soul_config(workspace_root, &patch)
}

pub fn update_traits(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    patch: PersonalityProfilePatch,
) -> Result<SoulConfig, ServiceError> {
    configure_workspace(deps, workspace_root, patch.into())
}

pub fn record_interaction(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    request: InteractionRecordRequest,
) -> Result<InteractionRecordResult, ServiceError> {
    deps.record_interaction(workspace_root, &request)
}

pub fn reset_adaptation_state(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    request: AdaptiveResetRequest,
) -> Result<AdaptiveResetResult, ServiceError> {
    deps.reset_adaptation_state(workspace_root, &request)
}
