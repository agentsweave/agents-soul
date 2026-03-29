use crate::{
    app::{
        deps::SoulDependencies,
        errors::{SoulHttpErrorResponse, SoulTransportError, map_soul_error},
    },
    domain::{BehavioralContext, ComposeRequest, SoulError},
    services::ServiceError,
};

pub fn compose_context(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<BehavioralContext, ServiceError> {
    deps.compose_context(request)
}

pub fn map_compose_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn compose_error_response(error: &SoulError) -> SoulHttpErrorResponse {
    map_compose_error(error).http_response()
}
