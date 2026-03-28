use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::domain::{ComposeRequest, ReputationSummary, SoulError, VerificationResult};

const VERIFICATION_CANDIDATES: [&str; 4] = [
    "registry_verification.json",
    "verification_result.json",
    ".soul/registry_verification.json",
    ".soul/verification_result.json",
];

const REPUTATION_CANDIDATES: [&str; 4] = [
    "registry_reputation.json",
    "reputation_summary.json",
    ".soul/registry_reputation.json",
    ".soul/reputation_summary.json",
];

#[derive(Debug, Clone, Default)]
pub struct RegistryReader;

impl RegistryReader {
    pub fn verify(&self, request: &ComposeRequest) -> Result<VerificationResult, SoulError> {
        let path = self
            .find_candidate(&request.workspace_id, &VERIFICATION_CANDIDATES)
            .ok_or(SoulError::RegistryUnavailable)?;
        self.read_verification_path(path)
    }

    pub fn reputation(&self, request: &ComposeRequest) -> Result<ReputationSummary, SoulError> {
        let path = self
            .find_candidate(&request.workspace_id, &REPUTATION_CANDIDATES)
            .ok_or(SoulError::RegistryUnavailable)?;
        self.read_reputation_path(path)
    }

    pub fn read_verification_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<VerificationResult, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::RegistryUnavailable)?;
        self.parse_verification(&content)
    }

    pub fn read_reputation_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ReputationSummary, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::RegistryUnavailable)?;
        self.parse_reputation(&content)
    }

    pub fn parse_verification(&self, content: &str) -> Result<VerificationResult, SoulError> {
        serde_json::from_str(content).map_err(|error| {
            SoulError::InvalidConfig(format!("invalid registry verification payload: {error}"))
        })
    }

    pub fn parse_reputation(&self, content: &str) -> Result<ReputationSummary, SoulError> {
        serde_json::from_str(content).map_err(|error| {
            SoulError::InvalidConfig(format!("invalid registry reputation payload: {error}"))
        })
    }

    fn find_candidate(&self, workspace_id: &str, candidates: &[&str]) -> Option<PathBuf> {
        let root = Path::new(workspace_id);

        candidates
            .iter()
            .map(|candidate| root.join(candidate))
            .find(|candidate| candidate.is_file())
    }
}
