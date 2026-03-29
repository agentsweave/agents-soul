use crate::{
    app::{
        deps::SoulDependencies,
        errors::{SoulHttpErrorResponse, SoulTransportError, map_soul_error},
    },
    domain::{ComposeRequest, SoulError},
    services::{ServiceError, explain::ExplainReport},
};

pub fn explain_report(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<ExplainReport, ServiceError> {
    deps.explain_report(request)
}

pub fn map_explain_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn explain_error_response(error: &SoulError) -> SoulHttpErrorResponse {
    map_explain_error(error).http_response()
}
