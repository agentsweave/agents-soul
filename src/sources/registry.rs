use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{ComposeRequest, InputProvenance, ReputationSummary, SoulError, VerificationResult},
    sources::{ReaderSelection, cache::read_cached_inputs},
};

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistryReader;

impl RegistryReader {
    pub fn load_verification(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<VerificationResult>, SoulError> {
        if let Some(path) = request.registry_verification_path.as_ref() {
            let verification = self.read_verification_path(path)?;
            return Ok(ReaderSelection::loaded(
                verification,
                InputProvenance::explicit(path.clone()),
            ));
        }

        if let Some(path) = self.find_candidate(&request.workspace_id, &VERIFICATION_CANDIDATES) {
            let verification = self.read_verification_path(&path)?;
            return Ok(ReaderSelection::loaded(
                verification,
                InputProvenance::live(path.display().to_string()),
            ));
        }

        let cached = read_cached_inputs(request)?;
        if let Some(verification) = cached
            .cached_inputs
            .as_ref()
            .and_then(|cached_inputs| cached_inputs.verification_result.clone())
        {
            let mut selection = ReaderSelection::loaded(
                verification,
                InputProvenance::cache(
                    crate::app::config::WorkspacePaths::new(&request.workspace_id)
                        .context_cache_path()
                        .display()
                        .to_string(),
                ),
            );
            selection.warnings.extend(cached.warnings);
            return Ok(selection);
        }

        let mut selection = ReaderSelection::unavailable(InputProvenance::unavailable(
            "registry verification unavailable",
        ));
        selection.warnings.extend(cached.warnings);
        Ok(selection)
    }

    pub fn load_reputation(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<ReputationSummary>, SoulError> {
        if let Some(path) = request.registry_reputation_path.as_ref() {
            let reputation = self.read_reputation_path(path)?;
            return Ok(ReaderSelection::loaded(
                reputation,
                InputProvenance::explicit(path.clone()),
            ));
        }

        if let Some(path) = self.find_candidate(&request.workspace_id, &REPUTATION_CANDIDATES) {
            let reputation = self.read_reputation_path(&path)?;
            return Ok(ReaderSelection::loaded(
                reputation,
                InputProvenance::live(path.display().to_string()),
            ));
        }

        let cached = read_cached_inputs(request)?;
        if let Some(reputation) = cached
            .cached_inputs
            .as_ref()
            .and_then(|cached_inputs| cached_inputs.reputation_summary.clone())
        {
            let mut selection = ReaderSelection::loaded(
                reputation,
                InputProvenance::cache(
                    crate::app::config::WorkspacePaths::new(&request.workspace_id)
                        .context_cache_path()
                        .display()
                        .to_string(),
                ),
            );
            selection.warnings.extend(cached.warnings);
            return Ok(selection);
        }

        let mut selection = ReaderSelection::unavailable(InputProvenance::unavailable(
            "registry reputation unavailable",
        ));
        selection.warnings.extend(cached.warnings);
        Ok(selection)
    }

    pub fn verify(&self, request: &ComposeRequest) -> Result<VerificationResult, SoulError> {
        let path = request
            .registry_verification_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| self.find_candidate(&request.workspace_id, &VERIFICATION_CANDIDATES))
            .ok_or(SoulError::RegistryUnavailable)?;
        self.read_verification_path(path)
    }

    pub fn reputation(&self, request: &ComposeRequest) -> Result<ReputationSummary, SoulError> {
        let path = request
            .registry_reputation_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| self.find_candidate(&request.workspace_id, &REPUTATION_CANDIDATES))
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
        serde_json::from_str(content).map_err(|error| SoulError::UpstreamInvalid {
            input: "registry-verification",
            message: error.to_string(),
        })
    }

    pub fn parse_reputation(&self, content: &str) -> Result<ReputationSummary, SoulError> {
        serde_json::from_str(content).map_err(|error| SoulError::UpstreamInvalid {
            input: "registry-reputation",
            message: error.to_string(),
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
