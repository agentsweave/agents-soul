use std::path::PathBuf;

use crate::{
    app::deps::SoulDependencies,
    domain::{DecisionHeuristicPatch, SoulConfig},
    services::ServiceError,
};

pub fn update_heuristics(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    patch: DecisionHeuristicPatch,
) -> Result<SoulConfig, ServiceError> {
    deps.update_soul_config(workspace_root, &patch.into())
}
