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

pub use api::{
    COMPOSE_ROUTE, EXPLAIN_ROUTE, HttpRequest, HttpResponse, RECORD_INTERACTION_ROUTE,
    RESET_ADAPTATION_ROUTE, UPDATE_HEURISTICS_ROUTE, UPDATE_TRAITS_ROUTE, handle_request,
};
pub use app::{
    deps::{AppDeps, SoulDependencies, SourceDependencies},
    errors::{
        SoulHttpError, SoulHttpErrorBody, SoulHttpErrorResponse, SoulMcpToolError,
        SoulMcpToolErrorData, SoulTransportError, compose_mode_hint_for, map_soul_error,
    },
    runtime::{CrateLayer, SoulRuntime, core_layers, crate_layout, transport_layers},
};
pub use domain::*;
