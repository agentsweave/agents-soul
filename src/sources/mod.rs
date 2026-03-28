use crate::domain::{BehaviorWarning, InputProvenance};

pub mod cache;
pub mod identity;
pub mod normalize;
pub mod registry;

#[derive(Debug, Clone, PartialEq)]
pub struct ReaderSelection<T> {
    pub value: Option<T>,
    pub provenance: InputProvenance,
    pub warnings: Vec<BehaviorWarning>,
}

impl<T> ReaderSelection<T> {
    pub fn loaded(value: T, provenance: InputProvenance) -> Self {
        Self {
            value: Some(value),
            provenance,
            warnings: Vec::new(),
        }
    }

    pub fn unavailable(provenance: InputProvenance) -> Self {
        Self {
            value: None,
            provenance,
            warnings: Vec::new(),
        }
    }
}
