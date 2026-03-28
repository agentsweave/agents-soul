use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::status::ComposeMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SoulErrorCategory {
    InvalidConfig,
    UpstreamUnavailable,
    RevokedStanding,
    StorageFailure,
    TemplateFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulTransportError {
    pub code: &'static str,
    pub category: SoulErrorCategory,
    pub message: String,
    pub compose_mode_hint: Option<ComposeMode>,
    pub http_status: u16,
    pub cli_exit_code: u8,
    pub mcp_error_code: i32,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SoulError {
    #[error("failed to read soul config at `{path}`: {message}")]
    ConfigRead { path: String, message: String },
    #[error("failed to parse soul config at `{path}`: {message}")]
    ConfigParse { path: String, message: String },
    #[error("invalid soul config: {0}")]
    InvalidConfig(String),
    #[error("field `{field}` must be within 0.0..=1.0, got {value}")]
    InvalidTraitValue { field: &'static str, value: f32 },
    #[error("field `{0}` must not be empty")]
    EmptyField(&'static str),
    #[error("duplicate heuristic id `{0}`")]
    DuplicateHeuristicId(String),
    #[error("required upstream inputs are broken")]
    RequiredInputsBroken,
    #[error("identity input unavailable")]
    IdentityUnavailable,
    #[error("registry verification unavailable")]
    RegistryUnavailable,
    #[error("revoked standing cannot compose normal context")]
    RevokedStanding,
    #[error("storage error: {0}")]
    Storage(String),
    #[error("failed to load template `{template}`: {message}")]
    TemplateLoad {
        template: &'static str,
        message: String,
    },
    #[error("failed to render template `{template}`: {message}")]
    TemplateRender {
        template: &'static str,
        message: String,
    },
}

impl SoulError {
    pub fn category(&self) -> SoulErrorCategory {
        match self {
            Self::ConfigRead { .. }
            | Self::ConfigParse { .. }
            | Self::InvalidConfig(_)
            | Self::InvalidTraitValue { .. }
            | Self::EmptyField(_)
            | Self::DuplicateHeuristicId(_) => SoulErrorCategory::InvalidConfig,
            Self::RequiredInputsBroken | Self::IdentityUnavailable | Self::RegistryUnavailable => {
                SoulErrorCategory::UpstreamUnavailable
            }
            Self::RevokedStanding => SoulErrorCategory::RevokedStanding,
            Self::Storage(_) => SoulErrorCategory::StorageFailure,
            Self::TemplateLoad { .. } | Self::TemplateRender { .. } => {
                SoulErrorCategory::TemplateFailure
            }
        }
    }

    pub fn compose_mode_hint(&self) -> Option<ComposeMode> {
        match self.category() {
            SoulErrorCategory::UpstreamUnavailable => Some(ComposeMode::Degraded),
            SoulErrorCategory::RevokedStanding => Some(ComposeMode::FailClosed),
            SoulErrorCategory::InvalidConfig
            | SoulErrorCategory::StorageFailure
            | SoulErrorCategory::TemplateFailure => None,
        }
    }

    pub fn transport_error(&self) -> SoulTransportError {
        let (code, http_status, cli_exit_code, mcp_error_code) = match self.category() {
            SoulErrorCategory::InvalidConfig => ("invalid-config", 400, 2, 1001),
            SoulErrorCategory::UpstreamUnavailable => ("upstream-unavailable", 503, 3, 1002),
            SoulErrorCategory::RevokedStanding => ("revoked-standing", 403, 4, 1003),
            SoulErrorCategory::StorageFailure => ("storage-failure", 500, 5, 1004),
            SoulErrorCategory::TemplateFailure => ("template-failure", 500, 6, 1005),
        };

        SoulTransportError {
            code,
            category: self.category(),
            message: self.to_string(),
            compose_mode_hint: self.compose_mode_hint(),
            http_status,
            cli_exit_code,
            mcp_error_code,
        }
    }
}
