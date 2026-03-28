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

pub use domain::*;
