use crate::{
    domain::{BehavioralContext, ComposeRequest, SoulError},
    services::SoulServices,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpTools;

pub fn compose_context(
    services: &SoulServices,
    request: ComposeRequest,
) -> Result<BehavioralContext, SoulError> {
    services.compose.compose(request)
}
