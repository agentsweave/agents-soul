use crate::{
    api::{
        compose::ComposeEndpoint, explain::ExplainEndpoint, heuristics::HeuristicsEndpoint,
        interactions::InteractionsEndpoint, reset::ResetEndpoint, traits::TraitsEndpoint,
    },
    services::SoulServices,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiRouter {
    pub compose: ComposeEndpoint,
    pub traits: TraitsEndpoint,
    pub heuristics: HeuristicsEndpoint,
    pub interactions: InteractionsEndpoint,
    pub reset: ResetEndpoint,
    pub explain: ExplainEndpoint,
}

impl ApiRouter {
    pub fn from_services(_services: &SoulServices) -> Self {
        Self::default()
    }
}
