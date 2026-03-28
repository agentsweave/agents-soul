use crate::{
    app::deps::SoulDependencies,
    domain::{BehavioralContext, ComposeRequest, SoulError, SoulTransportError},
    services::ServiceError,
};

pub fn compose_context(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<BehavioralContext, ServiceError> {
    deps.compose_context(request)
}

pub fn map_compose_error(error: &SoulError) -> SoulTransportError {
    error.transport_error()
}
