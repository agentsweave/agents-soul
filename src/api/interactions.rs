use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    adaptation::{InteractionRecordEffect, InteractionRecordRequest, InteractionRecordResult},
    app::{
        deps::SoulDependencies,
        errors::{SoulHttpErrorResponse, SoulTransportError, map_soul_error},
    },
    domain::{InteractionEvent, InteractionOutcome, InteractionSignal, SoulError},
    services::ServiceError,
};

pub fn record_interaction(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    request: InteractionRecordRequest,
) -> Result<InteractionRecordResult, ServiceError> {
    deps.record_interaction(workspace_root, &request)
}

pub fn map_record_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn record_error_response(error: &SoulError) -> SoulHttpErrorResponse {
    map_record_error(error).http_response()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordInteractionRequest {
    pub workspace_root: String,
    pub event_id: String,
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub interaction_type: String,
    #[serde(default)]
    pub outcome: InteractionOutcome,
    #[serde(default)]
    pub signals: Vec<InteractionSignal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub context: Value,
    #[serde(default = "default_persist")]
    pub persist: bool,
}

fn default_persist() -> bool {
    true
}

impl RecordInteractionRequest {
    fn validate(&self) -> Result<(), SoulError> {
        if self.workspace_root.trim().is_empty() {
            return Err(SoulError::Validation(
                "workspace_root must not be empty".to_owned(),
            ));
        }
        if self.event_id.trim().is_empty() {
            return Err(SoulError::Validation(
                "event_id must not be empty".to_owned(),
            ));
        }
        if self.agent_id.trim().is_empty() {
            return Err(SoulError::Validation(
                "agent_id must not be empty".to_owned(),
            ));
        }
        if self.interaction_type.trim().is_empty() {
            return Err(SoulError::Validation(
                "interaction_type must not be empty".to_owned(),
            ));
        }
        Ok(())
    }

    fn into_domain_request(self) -> Result<(String, InteractionRecordRequest), SoulError> {
        self.validate()?;
        let workspace_root = self.workspace_root.clone();
        Ok((
            workspace_root,
            InteractionRecordRequest {
                event_id: self.event_id,
                event: InteractionEvent {
                    agent_id: self.agent_id,
                    session_id: self.session_id,
                    interaction_type: self.interaction_type,
                    outcome: self.outcome,
                    signals: self.signals,
                    notes: self.notes,
                    recorded_at: self.recorded_at.unwrap_or_else(Utc::now),
                },
                context_json: self.context.to_string(),
                persist: self.persist,
            },
        ))
    }
}

pub fn handle_record_interaction(
    deps: &SoulDependencies,
    request: RecordInteractionRequest,
) -> Result<InteractionRecordResult, SoulError> {
    let (workspace_root, domain_request) = request.into_domain_request()?;
    record_interaction(deps, workspace_root, domain_request)
}

pub fn record_success_status(result: &InteractionRecordResult) -> u16 {
    match result.effect {
        InteractionRecordEffect::Duplicate => 200,
        InteractionRecordEffect::SessionOnly
        | InteractionRecordEffect::Inserted
        | InteractionRecordEffect::Updated
        | InteractionRecordEffect::Unchanged => 201,
    }
}
