use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::{
        BehaviorWarning, ComposeRequest, InputProvenance, SessionIdentitySnapshot, SoulConfig,
        SoulError, WarningSeverity,
    },
    sources::{ReaderSelection, cache::read_cached_inputs},
};

const IDENTITY_CANDIDATES: [&str; 4] = [
    "session_identity_snapshot.json",
    "identity_snapshot.json",
    ".soul/session_identity_snapshot.json",
    ".soul/identity_snapshot.json",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityReader;

impl IdentityReader {
    pub fn load(
        &self,
        request: &ComposeRequest,
        config: &SoulConfig,
    ) -> Result<ReaderSelection<SessionIdentitySnapshot>, SoulError> {
        if let Some(path) = request.identity_snapshot_path.as_ref() {
            let snapshot = self.read_snapshot_path(path)?;
            return Ok(ReaderSelection::loaded(
                snapshot,
                InputProvenance::explicit(path.clone()),
            ));
        }

        if let Some(path) = self.find_snapshot_path(&config.sources.identity_workspace) {
            let snapshot = self.read_snapshot_path(&path)?;
            return Ok(ReaderSelection::loaded(
                snapshot,
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
                snapshot,
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
        let path = request
            .identity_snapshot_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| self.find_snapshot_path(&request.workspace_id))
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
        let snapshot: SessionIdentitySnapshot =
            serde_json::from_str(content).map_err(|error| SoulError::UpstreamInvalid {
                input: "identity-snapshot",
                message: error.to_string(),
            })?;

        if snapshot.agent_id.trim().is_empty() {
            return Err(SoulError::UpstreamInvalid {
                input: "identity-snapshot",
                message: "field `identity_snapshot.agent_id` must not be empty".into(),
            });
        }

        Ok(snapshot)
    }

    fn find_snapshot_path(&self, workspace_id: &str) -> Option<PathBuf> {
        let root = expand_root(workspace_id);

        IDENTITY_CANDIDATES
            .iter()
            .map(|candidate| root.join(candidate))
            .find(|candidate| candidate.is_file())
    }
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
