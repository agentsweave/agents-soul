//! Compatibility module for behavioral context contract types.
//!
//! The canonical definitions live in `behavioral_context.rs`, `provenance.rs`, and
//! `status.rs`. This module provides the historical grouping without allowing the
//! contract to drift into duplicate type definitions.

pub use super::behavioral_context::{
    BehaviorWarning as BehavioralWarning, BehavioralContext, WarningSeverity,
};
pub use super::provenance::ProvenanceReport;
pub use super::status::{RecoveryState, RegistryStatus, StatusSummary};
