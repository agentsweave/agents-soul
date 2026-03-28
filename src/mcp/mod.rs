pub mod server;
pub mod tools;

use crate::{mcp::server::McpServer, services::SoulServices};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpSurface {
    pub server: McpServer,
}

impl McpSurface {
    pub fn from_services(services: &SoulServices) -> Self {
        Self {
            server: McpServer::from_services(services),
        }
    }
}
