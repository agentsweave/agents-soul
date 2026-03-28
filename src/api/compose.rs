use crate::{
    domain::{BehavioralContext, ComposeRequest},
    services::{ServiceError, SoulServices},
};

pub fn compose_context(
    services: &SoulServices,
    request: ComposeRequest,
) -> Result<BehavioralContext, ServiceError> {
    services.compose.compose(request)
}
