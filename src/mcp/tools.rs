use crate::{
    app::{
        deps::SoulDependencies,
        errors::{SoulMcpToolError, SoulTransportError, map_soul_error},
    },
    domain::{
        BehavioralContext, ComposeRequest, PersonalityProfilePatch, SoulConfig, SoulConfigPatch,
        SoulError,
    },
    services::ServiceError,
};

pub fn compose_context(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<BehavioralContext, ServiceError> {
    deps.compose_context(request)
}

pub fn map_compose_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn compose_tool_error(error: &SoulError) -> SoulMcpToolError {
    map_compose_error(error).mcp_tool_error()
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
    workspace_root: impl Into<std::path::PathBuf>,
    patch: PersonalityProfilePatch,
) -> Result<SoulConfig, ServiceError> {
    configure_workspace(deps, workspace_root, patch.into())
}
