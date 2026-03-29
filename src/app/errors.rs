use std::process::ExitCode;

use serde::Serialize;

use crate::domain::{ComposeMode, SoulError, SoulErrorCategory};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoulTransportError {
    pub code: &'static str,
    pub category: SoulErrorCategory,
    pub message: String,
    pub compose_mode_hint: Option<ComposeMode>,
    pub http_status: u16,
    pub cli_exit_code: u8,
    pub mcp_error_code: i32,
    pub mcp_error_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoulHttpErrorResponse {
    pub status: u16,
    pub body: SoulHttpErrorBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoulHttpErrorBody {
    pub error: SoulHttpError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoulHttpError {
    pub code: &'static str,
    pub category: SoulErrorCategory,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose_mode_hint: Option<ComposeMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoulMcpToolError {
    pub code: &'static str,
    pub message: String,
    pub data: SoulMcpToolErrorData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoulMcpToolErrorData {
    pub error_code: i32,
    pub category: SoulErrorCategory,
    pub http_status: u16,
    pub cli_exit_code: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose_mode_hint: Option<ComposeMode>,
}

impl SoulTransportError {
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.cli_exit_code)
    }

    pub fn http_response(&self) -> SoulHttpErrorResponse {
        SoulHttpErrorResponse {
            status: self.http_status,
            body: SoulHttpErrorBody {
                error: SoulHttpError {
                    code: self.code,
                    category: self.category,
                    message: self.message.clone(),
                    compose_mode_hint: self.compose_mode_hint,
                },
            },
        }
    }

    pub fn mcp_tool_error(&self) -> SoulMcpToolError {
        SoulMcpToolError {
            code: self.mcp_error_name,
            message: self.message.clone(),
            data: SoulMcpToolErrorData {
                error_code: self.mcp_error_code,
                category: self.category,
                http_status: self.http_status,
                cli_exit_code: self.cli_exit_code,
                compose_mode_hint: self.compose_mode_hint,
            },
        }
    }
}

pub fn compose_mode_hint_for(error: &SoulError) -> Option<ComposeMode> {
    match error.category() {
        SoulErrorCategory::UpstreamUnavailable => Some(ComposeMode::Degraded),
        SoulErrorCategory::FailClosed => Some(ComposeMode::FailClosed),
        SoulErrorCategory::LocalConfig
        | SoulErrorCategory::RequestValidation
        | SoulErrorCategory::UpstreamInvalid
        | SoulErrorCategory::StorageFailure
        | SoulErrorCategory::TemplateFailure
        | SoulErrorCategory::InternalFailure => None,
    }
}

pub fn map_soul_error(error: &SoulError) -> SoulTransportError {
    let category = error.category();
    let (code, http_status, cli_exit_code, mcp_error_code, mcp_error_name) = match category {
        SoulErrorCategory::RequestValidation => (
            "request-validation",
            400,
            2,
            1001,
            "soul/request-validation",
        ),
        SoulErrorCategory::UpstreamUnavailable => (
            "upstream-unavailable",
            503,
            3,
            1002,
            "soul/upstream-unavailable",
        ),
        SoulErrorCategory::FailClosed => ("fail-closed", 403, 4, 1003, "soul/fail-closed"),
        SoulErrorCategory::LocalConfig => (
            "local-config-invalid",
            500,
            5,
            1004,
            "soul/local-config-invalid",
        ),
        SoulErrorCategory::StorageFailure => {
            ("storage-failure", 500, 6, 1005, "soul/storage-failure")
        }
        SoulErrorCategory::UpstreamInvalid => {
            ("upstream-invalid", 502, 7, 1006, "soul/upstream-invalid")
        }
        SoulErrorCategory::TemplateFailure => {
            ("template-failure", 500, 7, 1007, "soul/template-failure")
        }
        SoulErrorCategory::InternalFailure => {
            ("internal-failure", 500, 7, 1008, "soul/internal-failure")
        }
    };

    SoulTransportError {
        code,
        category,
        message: error.to_string(),
        compose_mode_hint: compose_mode_hint_for(error),
        http_status,
        cli_exit_code,
        mcp_error_code,
        mcp_error_name,
    }
}
