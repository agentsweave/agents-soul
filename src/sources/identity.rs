use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::domain::{ComposeRequest, SessionIdentitySnapshot, SoulError};

const IDENTITY_CANDIDATES: [&str; 4] = [
    "session_identity_snapshot.json",
    "identity_snapshot.json",
    ".soul/session_identity_snapshot.json",
    ".soul/identity_snapshot.json",
];

#[derive(Debug, Clone, Default)]
pub struct IdentityReader;

impl IdentityReader {
    pub fn read_snapshot(
        &self,
        request: &ComposeRequest,
    ) -> Result<SessionIdentitySnapshot, SoulError> {
        let path = self
            .find_snapshot_path(&request.workspace_id)
            .ok_or(SoulError::IdentityUnavailable)?;
        self.read_snapshot_path(path)
    }

    pub fn read_snapshot_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<SessionIdentitySnapshot, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::IdentityUnavailable)?;
        self.parse_snapshot(&content)
    }

    pub fn parse_snapshot(&self, content: &str) -> Result<SessionIdentitySnapshot, SoulError> {
        let snapshot: SessionIdentitySnapshot = serde_json::from_str(content).map_err(|error| {
            SoulError::InvalidConfig(format!("invalid identity snapshot payload: {error}"))
        })?;

        if snapshot.agent_id.trim().is_empty() {
            return Err(SoulError::EmptyField("identity_snapshot.agent_id"));
        }

        Ok(snapshot)
    }

    fn find_snapshot_path(&self, workspace_id: &str) -> Option<PathBuf> {
        let root = Path::new(workspace_id);

        IDENTITY_CANDIDATES
            .iter()
            .map(|candidate| root.join(candidate))
            .find(|candidate| candidate.is_file())
    }
}
