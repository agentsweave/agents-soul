use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    app::config::WorkspacePaths,
    domain::{
        BehaviorWarning, ComposeRequest, ReputationSummary, SessionIdentitySnapshot, SoulError,
        VerificationResult, WarningSeverity,
    },
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CachedInputs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_snapshot: Option<SessionIdentitySnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_result: Option<VerificationResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reputation_summary: Option<ReputationSummary>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CacheReadResult {
    pub cached_inputs: Option<CachedInputs>,
    pub warnings: Vec<BehaviorWarning>,
}

pub fn read_cached_inputs(request: &ComposeRequest) -> Result<CacheReadResult, SoulError> {
    read_cached_inputs_path(WorkspacePaths::new(&request.workspace_id).context_cache_path())
}

pub fn read_cached_inputs_path(path: impl AsRef<Path>) -> Result<CacheReadResult, SoulError> {
    let path = path.as_ref();
    if !path.is_file() {
        return Ok(CacheReadResult::default());
    }

    let content =
        fs::read_to_string(path).map_err(|error| SoulError::Storage(error.to_string()))?;
    let cached_inputs = match serde_json::from_str::<CachedInputs>(&content) {
        Ok(cached_inputs) => Some(cached_inputs),
        Err(error) => {
            return Ok(CacheReadResult {
                cached_inputs: None,
                warnings: vec![BehaviorWarning {
                    severity: WarningSeverity::Important,
                    code: "context_cache_invalid".to_owned(),
                    message: format!(
                        "Context cache at `{}` is invalid and was bypassed: {error}",
                        path.display()
                    ),
                }],
            });
        }
    };

    Ok(CacheReadResult {
        cached_inputs,
        warnings: Vec::new(),
    })
}
