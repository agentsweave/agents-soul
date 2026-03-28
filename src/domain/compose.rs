//! Compatibility module for compose contract types.
//!
//! The canonical definitions live in `inputs.rs` and `status.rs`. This module keeps a
//! stable domain path without reintroducing shadow copies of the core runtime
//! contract.

pub use super::inputs::{ComposeRequest, NormalizedInputs};
pub use super::status::ComposeMode;
