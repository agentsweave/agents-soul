use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{
        ComposeRequest, InputProvenance, RegistryReputation, RegistrySnapshot, RegistryStanding,
        ReputationSummary, SoulError, VerificationResult,
    },
    sources::{ReaderSelection, cache::read_cached_inputs},
};

const REGISTRY_CANDIDATES: [&str; 4] = [
    "agents_registry.json",
    "registry_snapshot.json",
    ".soul/agents_registry.json",
    ".soul/registry_snapshot.json",
];

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
pub struct RegistryReader {
    pub real: RealRegistryAdapter,
    pub fixture: FixtureRegistryAdapter,
}

impl RegistryReader {
    pub fn load_verification(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<VerificationResult>, SoulError> {
        match self.real.load_verification(request)? {
            selection if selection.value.is_some() => Ok(selection),
            real_unavailable => self
                .fixture
                .load_verification(request)
                .map(|fixture| merge_unavailable(real_unavailable, fixture)),
        }
    }

    pub fn load_reputation(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<ReputationSummary>, SoulError> {
        match self.real.load_reputation(request)? {
            selection if selection.value.is_some() => Ok(selection),
            real_unavailable => self
                .fixture
                .load_reputation(request)
                .map(|fixture| merge_unavailable(real_unavailable, fixture)),
        }
    }

    pub fn load_snapshot(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<RegistrySnapshot>, SoulError> {
        match self.real.load_snapshot(request)? {
            selection if selection.value.is_some() => Ok(selection),
            real_unavailable => self
                .fixture
                .load_snapshot(request)
                .map(|fixture| merge_unavailable(real_unavailable, fixture)),
        }
    }

    pub fn verify(&self, request: &ComposeRequest) -> Result<VerificationResult, SoulError> {
        self.load_verification(request)?
            .value
            .ok_or(SoulError::RegistryUnavailable)
    }

    pub fn reputation(&self, request: &ComposeRequest) -> Result<ReputationSummary, SoulError> {
        self.load_reputation(request)?
            .value
            .ok_or(SoulError::RegistryUnavailable)
    }

    pub fn read_verification_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<VerificationResult, SoulError> {
        self.real.read_verification_path(path)
    }

    pub fn read_reputation_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ReputationSummary, SoulError> {
        self.real.read_reputation_path(path)
    }

    pub fn parse_verification(&self, content: &str) -> Result<VerificationResult, SoulError> {
        self.real.parse_verification(content)
    }

    pub fn parse_reputation(&self, content: &str) -> Result<ReputationSummary, SoulError> {
        self.real.parse_reputation(content)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RealRegistryAdapter;

impl RealRegistryAdapter {
    pub fn load_verification(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<RegistryStanding>, SoulError> {
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

        if let Some(path) = self.find_candidate(&request.workspace_id, &REGISTRY_CANDIDATES) {
            let snapshot = self.read_snapshot_path(&path)?;
            if let Some(standing) = snapshot.standing {
                return Ok(ReaderSelection::loaded(
                    standing,
                    InputProvenance::live(path.display().to_string()),
                ));
            }
        }

        Ok(ReaderSelection::unavailable(InputProvenance::unavailable(
            "registry verification unavailable",
        )))
    }

    pub fn load_reputation(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<RegistryReputation>, SoulError> {
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

        if let Some(path) = self.find_candidate(&request.workspace_id, &REGISTRY_CANDIDATES) {
            let snapshot = self.read_snapshot_path(&path)?;
            if let Some(reputation) = snapshot.reputation {
                return Ok(ReaderSelection::loaded(
                    reputation,
                    InputProvenance::live(path.display().to_string()),
                ));
            }
        }

        Ok(ReaderSelection::unavailable(InputProvenance::unavailable(
            "registry reputation unavailable",
        )))
    }

    pub fn load_snapshot(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<RegistrySnapshot>, SoulError> {
        if let Some(path) = self.find_candidate(&request.workspace_id, &REGISTRY_CANDIDATES) {
            let snapshot = self.read_snapshot_path(&path)?;
            return Ok(ReaderSelection::loaded(
                snapshot,
                InputProvenance::live(path.display().to_string()),
            ));
        }

        let standing_selection = self.load_verification(request)?;
        let reputation_selection = self.load_reputation(request)?;
        let standing = standing_selection.value;
        let reputation = reputation_selection.value;

        if standing.is_none() && reputation.is_none() {
            return Ok(ReaderSelection::unavailable(InputProvenance::unavailable(
                "registry snapshot unavailable",
            )));
        }

        let provenance = if matches!(
            standing_selection.provenance.source,
            crate::domain::InputSourceKind::Explicit
        ) || matches!(
            reputation_selection.provenance.source,
            crate::domain::InputSourceKind::Explicit
        ) {
            InputProvenance::explicit("registry split inputs")
        } else {
            InputProvenance::live("registry split inputs")
        };

        Ok(ReaderSelection::loaded(
            RegistrySnapshot {
                standing,
                reputation,
            },
            provenance,
        ))
    }

    pub fn read_verification_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RegistryStanding, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::RegistryUnavailable)?;
        self.parse_verification(&content)
    }

    pub fn read_reputation_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RegistryReputation, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::RegistryUnavailable)?;
        self.parse_reputation(&content)
    }

    pub fn read_snapshot_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<RegistrySnapshot, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::RegistryUnavailable)?;
        self.parse_snapshot(&content)
    }

    pub fn parse_verification(&self, content: &str) -> Result<RegistryStanding, SoulError> {
        serde_json::from_str(content).map_err(|error| SoulError::UpstreamInvalid {
            input: "agents-registry-standing",
            message: error.to_string(),
        })
    }

    pub fn parse_reputation(&self, content: &str) -> Result<RegistryReputation, SoulError> {
        serde_json::from_str(content).map_err(|error| SoulError::UpstreamInvalid {
            input: "agents-registry-reputation",
            message: error.to_string(),
        })
    }

    pub fn parse_snapshot(&self, content: &str) -> Result<RegistrySnapshot, SoulError> {
        serde_json::from_str(content).map_err(|error| SoulError::UpstreamInvalid {
            input: "agents-registry",
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixtureRegistryAdapter;

impl FixtureRegistryAdapter {
    pub fn load_verification(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<RegistryStanding>, SoulError> {
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
    ) -> Result<ReaderSelection<RegistryReputation>, SoulError> {
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

    pub fn load_snapshot(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<RegistrySnapshot>, SoulError> {
        let standing_selection = self.load_verification(request)?;
        let reputation_selection = self.load_reputation(request)?;
        let standing = standing_selection.value;
        let reputation = reputation_selection.value;
        let mut warnings = standing_selection.warnings;
        warnings.extend(reputation_selection.warnings);

        let provenance = if standing.is_some() || reputation.is_some() {
            InputProvenance::cache(
                crate::app::config::WorkspacePaths::new(&request.workspace_id)
                    .context_cache_path()
                    .display()
                    .to_string(),
            )
        } else {
            InputProvenance::unavailable("registry snapshot unavailable")
        };

        let mut selection = if standing.is_some() || reputation.is_some() {
            ReaderSelection::loaded(
                RegistrySnapshot {
                    standing,
                    reputation,
                },
                provenance,
            )
        } else {
            ReaderSelection::unavailable(provenance)
        };
        selection.warnings = warnings;
        Ok(selection)
    }
}

fn merge_unavailable<T>(
    mut primary: ReaderSelection<T>,
    fallback: ReaderSelection<T>,
) -> ReaderSelection<T> {
    primary.warnings.extend(fallback.warnings);
    if fallback.value.is_some() {
        ReaderSelection {
            value: fallback.value,
            provenance: fallback.provenance,
            warnings: primary.warnings,
        }
    } else {
        primary
    }
}
