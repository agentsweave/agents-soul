use crate::{
    adaptation::AdaptationStack, services::SoulServices, sources::SourceReaders,
    storage::StorageStack,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoulDependencies {
    pub sources: SourceReaders,
    pub adaptation: AdaptationStack,
    pub storage: StorageStack,
    pub services: SoulServices,
}
