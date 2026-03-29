use std::path::PathBuf;

use clap::Args;

use crate::{
    adaptation::{InteractionRecordRequest, InteractionRecordResult},
    app::{config::ApplicationConfig, deps::SoulDependencies},
    domain::{InteractionEvent, InteractionOutcome, SoulError},
    services::ServiceError,
};

#[derive(Debug, Clone, Args)]
pub struct RecordCmd {
    #[arg(long)]
    pub workspace: String,
    #[arg(long = "interaction-type")]
    pub interaction_type: String,
    #[arg(long)]
    pub outcome: String,
    #[arg(long)]
    pub notes: Option<String>,
}

pub fn record_interaction(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    request: InteractionRecordRequest,
) -> Result<InteractionRecordResult, ServiceError> {
    deps.record_interaction(workspace_root, &request)
}

pub fn record_cmd(
    deps: &SoulDependencies,
    _config: &ApplicationConfig,
    cmd: RecordCmd,
) -> Result<InteractionRecordResult, SoulError> {
    let workspace_root = PathBuf::from(&cmd.workspace);
    let config = deps.load_soul_config(&cmd.workspace)?;
    let recorded_at = deps.now();
    let request = InteractionRecordRequest {
        event_id: format!(
            "cli-record-{}-{}",
            config.agent_id,
            recorded_at.timestamp_nanos_opt().unwrap_or_default()
        ),
        event: InteractionEvent {
            agent_id: config.agent_id,
            session_id: None,
            interaction_type: cmd.interaction_type,
            outcome: parse_outcome(&cmd.outcome)?,
            signals: Vec::new(),
            notes: cmd.notes,
            recorded_at,
        },
        context_json: "{\"source\":\"cli\"}".to_owned(),
        persist: true,
    };

    record_interaction(deps, workspace_root, request)
}

fn parse_outcome(raw: &str) -> Result<InteractionOutcome, SoulError> {
    match raw.trim() {
        "positive" => Ok(InteractionOutcome::Positive),
        "neutral" => Ok(InteractionOutcome::Neutral),
        "negative" => Ok(InteractionOutcome::Negative),
        other => Err(SoulError::Validation(format!(
            "unsupported outcome `{other}`; expected positive, neutral, or negative"
        ))),
    }
}
