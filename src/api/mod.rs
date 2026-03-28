pub mod compose;
pub mod explain;
pub mod heuristics;
pub mod interactions;
pub mod reset;
pub mod router;
pub mod traits;

use crate::{api::router::ApiRouter, services::SoulServices};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiSurface {
    pub router: ApiRouter,
}

impl ApiSurface {
    pub fn from_services(services: &SoulServices) -> Self {
        Self {
            router: ApiRouter::from_services(services),
        }
    }
}
