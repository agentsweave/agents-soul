//! CLI reset command surface.

use std::path::PathBuf;

use clap::{Args, ValueEnum};

use crate::{
    adaptation::{AdaptiveResetRequest, AdaptiveResetResult},
    app::{config::ApplicationConfig, deps::SoulDependencies},
    domain::SoulError,
    services::ServiceError,
    storage::sqlite::ResetScope,
};

#[derive(Debug, Clone, Args)]
pub struct ResetCmd {
    #[arg(long)]
    pub workspace: String,
    #[arg(long, value_enum, default_value_t = ResetScopeArg::All)]
    pub scope: ResetScopeArg,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ResetScopeArg {
    #[default]
    All,
    Trait,
    Communication,
    Heuristic,
}

impl From<ResetScopeArg> for ResetScope {
    fn from(value: ResetScopeArg) -> Self {
        match value {
            ResetScopeArg::All => ResetScope::All,
            ResetScopeArg::Trait => ResetScope::Trait,
            ResetScopeArg::Communication => ResetScope::Communication,
            ResetScopeArg::Heuristic => ResetScope::Heuristic,
        }
    }
}

pub fn reset_adaptation_state(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    request: AdaptiveResetRequest,
) -> Result<AdaptiveResetResult, ServiceError> {
    deps.reset_adaptation_state(workspace_root, &request)
}

pub fn reset_cmd(
    deps: &SoulDependencies,
    _config: &ApplicationConfig,
    cmd: ResetCmd,
) -> Result<AdaptiveResetResult, SoulError> {
    let workspace_root = PathBuf::from(&cmd.workspace);
    let config = deps.load_soul_config(&cmd.workspace)?;
    let recorded_at = deps.now();
    let request = AdaptiveResetRequest {
        reset_id: format!(
            "cli-reset-{}-{}",
            config.agent_id,
            recorded_at.timestamp_nanos_opt().unwrap_or_default()
        ),
        agent_id: config.agent_id,
        scope: cmd.scope.into(),
        target_key: cmd.target,
        notes: cmd.notes,
        recorded_at,
    };

    reset_adaptation_state(deps, workspace_root, request)
}
