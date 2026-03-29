//! CLI configure command surface.

use std::path::PathBuf;

use crate::{
    app::deps::SoulDependencies,
    domain::{PersonalityProfilePatch, SoulConfig, SoulConfigPatch},
    services::ServiceError,
};

pub fn configure_workspace(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
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
