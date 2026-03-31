//! CLI compose command surface.

use clap::Args;
use serde::Serialize;

use crate::{
    app::{
        deps::SoulDependencies,
        errors::{SoulTransportError, map_soul_error},
    },
    domain::{BehavioralContext, ComposeRequest, SoulError},
    services::explain::FullContextReport,
};

#[derive(Debug, Clone, Args)]
pub struct ComposeCmd {
    #[arg(long)]
    pub workspace: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub prefix_only: bool,
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
    #[arg(long = "session-id", default_value = "cli.compose")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PromptPrefixOutput {
    pub system_prompt_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ComposeOutput {
    Context(BehavioralContext),
    Prefix(PromptPrefixOutput),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComposePresentation {
    pub rendered: Option<String>,
    pub output: ComposeOutput,
}

pub fn compose_cmd(
    deps: &SoulDependencies,
    cmd: ComposeCmd,
) -> Result<ComposePresentation, SoulError> {
    let request = build_request(deps, &cmd)?;
    if cmd.prefix_only {
        let context = deps.compose_context(request)?;
        let system_prompt_prefix = context.system_prompt_prefix;
        return Ok(ComposePresentation {
            rendered: (!cmd.json).then_some(system_prompt_prefix.clone()),
            output: ComposeOutput::Prefix(PromptPrefixOutput {
                system_prompt_prefix,
            }),
        });
    }

    if cmd.json {
        let context = deps.compose_context(request)?;
        Ok(ComposePresentation {
            rendered: None,
            output: ComposeOutput::Context(context),
        })
    } else {
        let report = deps.full_context_report(request)?;
        let FullContextReport {
            context, rendered, ..
        } = report;
        Ok(ComposePresentation {
            rendered: Some(rendered),
            output: ComposeOutput::Context(context),
        })
    }
}

pub fn build_request(
    deps: &SoulDependencies,
    cmd: &ComposeCmd,
) -> Result<ComposeRequest, SoulError> {
    let config = deps.load_soul_config(&cmd.workspace)?;
    Ok(ComposeRequest {
        workspace_id: cmd.workspace.clone(),
        agent_id: config.agent_id,
        session_id: cmd.session_id.clone(),
        identity_snapshot_path: cmd.identity_snapshot_path.clone(),
        registry_verification_path: cmd.registry_verification_path.clone(),
        registry_reputation_path: cmd.registry_reputation_path.clone(),
        include_reputation: !cmd.no_reputation,
        include_relationships: !cmd.no_relationships,
        include_commitments: !cmd.no_commitments,
    })
}

pub fn map_compose_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}
