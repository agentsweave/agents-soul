pub mod commitments;
pub mod communication;
pub mod compose;
pub mod decision_rules;
pub mod explain;
pub mod limits;
pub mod profile;
pub mod provenance;
pub mod relationships;
pub mod templates;
pub mod warnings;

use thiserror::Error;

pub use compose::ComposeService;

#[derive(Debug, Clone, Default)]
pub struct SoulServices {
    pub compose: ComposeService,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ServiceError {
    #[error("{0}")]
    InvalidRequest(crate::domain::SoulError),
}
