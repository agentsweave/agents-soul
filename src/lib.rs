#![forbid(unsafe_code)]

//! Crate boundary bootstrap for `agents-soul`.
//!
//! The domain and service layers are the canonical write surface. CLI, API, and MCP
//! stay thin and delegate into the shared services stack.

pub mod adaptation;
pub mod api;
pub mod app;
pub mod cli;
pub mod domain;
pub mod mcp;
pub mod services;
pub mod sources;
pub mod storage;

pub use app::{
    deps::{AppDeps, SoulDependencies, SourceDependencies},
    errors::{
        SoulHttpError, SoulHttpErrorBody, SoulHttpErrorResponse, SoulMcpToolError,
        SoulMcpToolErrorData, SoulTransportError, compose_mode_hint_for, map_soul_error,
    },
    runtime::{CrateLayer, SoulRuntime, core_layers, crate_layout, transport_layers},
};
pub use domain::*;
