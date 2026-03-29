use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{
        BehaviorWarning, ComposeRequest, IdentifySignals, InputProvenance, RecoveryState,
        SessionIdentitySnapshot, SoulConfig, SoulError, WarningSeverity,
    },
    sources::{ReaderSelection, cache::read_cached_inputs},
};

const IDENTITY_CANDIDATES: [&str; 4] = [
    "session_identity_snapshot.json",
    "identity_snapshot.json",
    ".soul/session_identity_snapshot.json",
    ".soul/identity_snapshot.json",
];

const IDENTIFY_CANDIDATES: [&str; 4] = [
    "agents_identify.json",
    "identify_snapshot.json",
    ".soul/agents_identify.json",
    ".soul/identify_snapshot.json",
];

pub trait IdentifyReaderContract: Send + Sync {
    fn load(
        &self,
        request: &ComposeRequest,
        config: &SoulConfig,
    ) -> Result<ReaderSelection<IdentifySignals>, SoulError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityReader;

impl IdentifyReaderContract for IdentityReader {
    fn load(
        &self,
        request: &ComposeRequest,
        config: &SoulConfig,
    ) -> Result<ReaderSelection<IdentifySignals>, SoulError> {
        IdentityReader::load(self, request, config)
    }
}

impl IdentityReader {
    pub fn load(
        &self,
        request: &ComposeRequest,
        config: &SoulConfig,
    ) -> Result<ReaderSelection<IdentifySignals>, SoulError> {
        if let Some(path) = request.identity_snapshot_path.as_ref() {
            let signals = self.read_signals_path(path)?;
            return Ok(ReaderSelection::loaded(
                signals,
                InputProvenance::explicit(path.clone()),
            ));
        }

        if let Some(path) = self.find_identify_path(&config.sources.identity_workspace) {
            let signals = self.read_signals_path(&path)?;
            return Ok(ReaderSelection::loaded(
                signals,
                InputProvenance::live(path.display().to_string()),
            ));
        }

        if let Some(path) = self.find_snapshot_path(&config.sources.identity_workspace) {
            let snapshot = self.read_snapshot_path(&path)?;
            return Ok(ReaderSelection::loaded(
                IdentifySignals {
                    recovery_state: Some(snapshot.recovery_state),
                    snapshot: Some(snapshot),
                },
                InputProvenance::live(path.display().to_string()),
            ));
        }

        let cached = read_cached_inputs(request)?;
        if let Some(snapshot) = cached
            .cached_inputs
            .as_ref()
            .and_then(|cached_inputs| cached_inputs.identity_snapshot.clone())
        {
            let mut selection = ReaderSelection::loaded(
                IdentifySignals {
                    recovery_state: Some(snapshot.recovery_state),
                    snapshot: Some(snapshot),
                },
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
            "identity snapshot unavailable",
        ));
        selection.warnings.extend(cached.warnings);
        Ok(selection)
    }

    pub fn read_snapshot(
        &self,
        request: &ComposeRequest,
    ) -> Result<SessionIdentitySnapshot, SoulError> {
        self.read_signals(request)?
            .snapshot
            .ok_or(SoulError::IdentityUnavailable)
    }

    pub fn read_signals(&self, request: &ComposeRequest) -> Result<IdentifySignals, SoulError> {
        let path = request
            .identity_snapshot_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| self.find_identify_path(&request.workspace_id))
            .or_else(|| self.find_snapshot_path(&request.workspace_id))
            .ok_or(SoulError::IdentityUnavailable)?;
        self.read_signals_path(path)
    }

    pub fn read_snapshot_path(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<SessionIdentitySnapshot, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::IdentityUnavailable)?;
        self.parse_snapshot(&content)
    }

    pub fn read_signals_path(&self, path: impl AsRef<Path>) -> Result<IdentifySignals, SoulError> {
        let content = fs::read_to_string(path).map_err(|_| SoulError::IdentityUnavailable)?;
        self.parse_signals(&content)
    }

    pub fn parse_snapshot(&self, content: &str) -> Result<SessionIdentitySnapshot, SoulError> {
        let snapshot: SessionIdentitySnapshot =
            serde_json::from_str(content).map_err(|error| SoulError::UpstreamInvalid {
                input: "identity-snapshot",
                message: error.to_string(),
            })?;

        validate_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub fn parse_signals(&self, content: &str) -> Result<IdentifySignals, SoulError> {
        match serde_json::from_str::<IdentifySignals>(content) {
            Ok(signals) => {
                validate_signals(&signals)?;
                Ok(signals)
            }
            Err(_) => {
                let snapshot = self.parse_snapshot(content)?;
                Ok(IdentifySignals {
                    recovery_state: Some(snapshot.recovery_state),
                    snapshot: Some(snapshot),
                })
            }
        }
    }

    fn find_snapshot_path(&self, workspace_id: &str) -> Option<PathBuf> {
        let root = expand_root(workspace_id);

        IDENTITY_CANDIDATES
            .iter()
            .map(|candidate| root.join(candidate))
            .find(|candidate| candidate.is_file())
    }

    fn find_identify_path(&self, workspace_id: &str) -> Option<PathBuf> {
        let root = expand_root(workspace_id);

        IDENTIFY_CANDIDATES
            .iter()
            .map(|candidate| root.join(candidate))
            .find(|candidate| candidate.is_file())
    }
}

fn validate_snapshot(snapshot: &SessionIdentitySnapshot) -> Result<(), SoulError> {
    if snapshot.agent_id.trim().is_empty() {
        return Err(SoulError::UpstreamInvalid {
            input: "identity-snapshot",
            message: "field `identity_snapshot.agent_id` must not be empty".into(),
        });
    }

    Ok(())
}

fn validate_signals(signals: &IdentifySignals) -> Result<(), SoulError> {
    if let Some(snapshot) = signals.snapshot.as_ref() {
        validate_snapshot(snapshot)?;
    }

    if signals.snapshot.is_none() && signals.recovery_state.is_none() {
        return Err(SoulError::UpstreamInvalid {
            input: "agents-identify",
            message: "identify signals must include at least one of `snapshot` or `recovery_state`"
                .into(),
        });
    }

    Ok(())
}

fn expand_root(raw_root: &str) -> PathBuf {
    if let Some(stripped) = raw_root.strip_prefix("~/")
        && let Some(home) = env::var_os("HOME")
    {
        return PathBuf::from(home).join(stripped);
    }

    Path::new(raw_root).to_path_buf()
}

pub fn agent_mismatch_warning(expected_agent_id: &str, actual_agent_id: &str) -> BehaviorWarning {
    BehaviorWarning {
        severity: WarningSeverity::Important,
        code: "identity_agent_mismatch".to_owned(),
        message: format!(
            "Identity snapshot agent `{actual_agent_id}` did not match requested agent `{expected_agent_id}` and was ignored."
        ),
    }
}

pub fn missing_snapshot_warning(recovery_state: RecoveryState) -> BehaviorWarning {
    BehaviorWarning {
        severity: WarningSeverity::Caution,
        code: "identify_snapshot_missing".to_owned(),
        message: format!(
            "agents-identify reported recovery state `{recovery_state:?}` without a usable snapshot; commitments and preferences were not loaded."
        ),
    }
}
