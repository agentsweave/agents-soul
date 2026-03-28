use crate::{
    domain::{BehavioralContext, ComposeRequest, SoulError, SoulTransportError},
    services::SoulServices,
    sources::{identity::IdentityReader, registry::RegistryReader},
};

#[derive(Debug, Clone, Default)]
pub struct SourceDependencies {
    pub identity: IdentityReader,
    pub registry: RegistryReader,
}

#[derive(Debug, Clone, Default)]
pub struct SoulDependencies {
    pub services: SoulServices,
    pub sources: SourceDependencies,
}

impl SoulDependencies {
    pub fn new(services: SoulServices, sources: SourceDependencies) -> Self {
        Self { services, sources }
    }

    pub fn compose_context(&self, request: ComposeRequest) -> Result<BehavioralContext, SoulError> {
        self.services.compose.compose(&self.sources, request)
    }

    pub fn map_error(&self, error: &SoulError) -> SoulTransportError {
        error.transport_error()
    }
}
