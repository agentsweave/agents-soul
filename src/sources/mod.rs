pub mod cache;
pub mod identity;
pub mod normalize;
pub mod registry;

use crate::sources::{
    cache::SourceCache, identity::IdentityReader, normalize::NormalizationPipeline,
    registry::RegistryReader,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceReaders {
    pub identity: IdentityReader,
    pub registry: RegistryReader,
    pub normalize: NormalizationPipeline,
    pub cache: SourceCache,
}
