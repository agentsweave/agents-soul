//! CLI explain command surface.

use clap::Args;
use serde::Serialize;

use crate::{
    app::{
        deps::SoulDependencies,
        errors::{map_soul_error, SoulTransportError},
    },
    domain::SoulError,
    services::explain::ExplainReport,
};

use super::compose::{build_request, ComposeCmd};

#[derive(Debug, Clone, Args)]
pub struct ExplainCmd {
    #[arg(long)]
    pub workspace: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long = "identity-snapshot")]
    pub identity_snapshot_path: Option<String>,
    #[arg(long = "registry-verification")]
    pub registry_verification_path: Option<String>,
    #[arg(long = "registry-reputation")]
    pub registry_reputation_path: Option<String>,
    #[arg(long = "no-reputation")]
    pub no_reputation: bool,
    #[arg(long = "no-relationships")]
    pub no_relationships: bool,
    #[arg(long = "no-commitments")]
    pub no_commitments: bool,
    #[arg(long = "session-id", default_value = "cli.explain")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExplainPresentation {
    pub rendered: Option<String>,
    pub output: ExplainReport,
}

pub fn explain_cmd(
    deps: &SoulDependencies,
    cmd: ExplainCmd,
) -> Result<ExplainPresentation, SoulError> {
    let json = cmd.json;
    let request = build_request(
        deps,
        &ComposeCmd {
            workspace: cmd.workspace,
            json,
            prefix_only: false,
            identity_snapshot_path: cmd.identity_snapshot_path,
            registry_verification_path: cmd.registry_verification_path,
            registry_reputation_path: cmd.registry_reputation_path,
            no_reputation: cmd.no_reputation,
            no_relationships: cmd.no_relationships,
            no_commitments: cmd.no_commitments,
            session_id: cmd.session_id,
        },
    )?;
    let report = deps.explain_report(request)?;
    let rendered = (!json).then_some(report.rendered.clone());

    Ok(ExplainPresentation {
        rendered,
        output: report,
    })
}

pub fn map_explain_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}
