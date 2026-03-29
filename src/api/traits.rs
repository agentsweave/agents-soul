use std::path::PathBuf;

use crate::{
    app::deps::SoulDependencies,
    domain::{PersonalityProfilePatch, SoulConfig},
    services::ServiceError,
};

pub fn update_traits(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    patch: PersonalityProfilePatch,
) -> Result<SoulConfig, ServiceError> {
    deps.update_soul_config(workspace_root, &patch.into())
}
