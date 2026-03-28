use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationConfig {
    pub workspace_root: PathBuf,
    pub enable_api: bool,
    pub enable_mcp: bool,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            enable_api: true,
            enable_mcp: true,
        }
    }
}
