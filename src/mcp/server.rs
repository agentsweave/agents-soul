use crate::{mcp::tools::McpTools, services::SoulServices};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpServer {
    pub tools: McpTools,
}

impl McpServer {
    pub fn from_services(_services: &SoulServices) -> Self {
        Self::default()
    }
}
