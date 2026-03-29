use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SoulErrorCategory {
    LocalConfig,
    RequestValidation,
    UpstreamUnavailable,
    UpstreamInvalid,
    FailClosed,
    StorageFailure,
    TemplateFailure,
    InternalFailure,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SoulError {
    #[error("failed to read soul config at `{path}`: {message}")]
    ConfigRead { path: String, message: String },
    #[error("failed to parse soul config at `{path}`: {message}")]
    ConfigParse { path: String, message: String },
    #[error("invalid soul config: {0}")]
    InvalidConfig(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("field `{field}` must be within 0.0..=1.0, got {value}")]
    InvalidTraitValue { field: &'static str, value: f32 },
    #[error("field `{0}` must not be empty")]
    EmptyField(&'static str),
    #[error("duplicate heuristic id `{0}`")]
    DuplicateHeuristicId(String),
    #[error("required upstream inputs are broken")]
    RequiredInputsBroken,
    #[error("upstream input `{input}` is invalid: {message}")]
    UpstreamInvalid {
        input: &'static str,
        message: String,
    },
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
    #[error("internal runtime failure: {0}")]
    Internal(String),
}

impl SoulError {
    pub fn category(&self) -> SoulErrorCategory {
        match self {
            Self::ConfigRead { .. } | Self::ConfigParse { .. } | Self::InvalidConfig(_) => {
                SoulErrorCategory::LocalConfig
            }
            Self::Validation(_)
            | Self::InvalidTraitValue { .. }
            | Self::EmptyField(_)
            | Self::DuplicateHeuristicId(_) => SoulErrorCategory::RequestValidation,
            Self::IdentityUnavailable | Self::RegistryUnavailable => {
                SoulErrorCategory::UpstreamUnavailable
            }
            Self::RequiredInputsBroken | Self::UpstreamInvalid { .. } => {
                SoulErrorCategory::UpstreamInvalid
            }
            Self::RevokedStanding => SoulErrorCategory::FailClosed,
            Self::Storage(_) => SoulErrorCategory::StorageFailure,
            Self::TemplateLoad { .. } | Self::TemplateRender { .. } => {
                SoulErrorCategory::TemplateFailure
            }
            Self::Internal(_) => SoulErrorCategory::InternalFailure,
        }
    }
}
