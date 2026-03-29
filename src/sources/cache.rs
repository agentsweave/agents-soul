use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    app::{config::WorkspacePaths, hash::stable_hash},
    domain::{
        BehaviorWarning, ComposeRequest, ReputationSummary, SessionIdentitySnapshot, SoulError,
        VerificationResult, WarningSeverity,
    },
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CachedInputs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<String>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileContextCache;

impl FileContextCache {
    pub fn read(&self, request: &ComposeRequest) -> Result<CacheReadResult, SoulError> {
        let cache_key = context_cache_key(request);
        read_cached_inputs_path_with_key(
            WorkspacePaths::new(&request.workspace_id).context_cache_path(),
            Some(cache_key.as_str()),
        )
    }

    pub fn write(
        &self,
        request: &ComposeRequest,
        cached_inputs: &CachedInputs,
    ) -> Result<(), SoulError> {
        let cache_key = context_cache_key(request);
        write_cached_inputs_path_with_key(
            WorkspacePaths::new(&request.workspace_id).context_cache_path(),
            cached_inputs,
            cache_key.as_str(),
        )
    }
}

pub fn context_cache_key(request: &ComposeRequest) -> String {
    // Intentionally exclude session_id so cache reuse follows stable source provenance.
    let key_seed = (
        request.workspace_id.as_str(),
        request.agent_id.as_str(),
        request.identity_snapshot_path.as_deref(),
        request.registry_verification_path.as_deref(),
        request.registry_reputation_path.as_deref(),
        request.include_reputation,
        request.include_relationships,
        request.include_commitments,
    );
    format!("ctx_{:016x}", stable_hash(key_seed))
}

pub fn read_cached_inputs(request: &ComposeRequest) -> Result<CacheReadResult, SoulError> {
    FileContextCache.read(request)
}

pub fn write_cached_inputs(
    request: &ComposeRequest,
    cached_inputs: &CachedInputs,
) -> Result<(), SoulError> {
    FileContextCache.write(request, cached_inputs)
}

pub fn read_cached_inputs_path(path: impl AsRef<Path>) -> Result<CacheReadResult, SoulError> {
    read_cached_inputs_path_with_key(path, None)
}

fn read_cached_inputs_path_with_key(
    path: impl AsRef<Path>,
    expected_cache_key: Option<&str>,
) -> Result<CacheReadResult, SoulError> {
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

    let Some(cached_inputs) = cached_inputs else {
        return Ok(CacheReadResult::default());
    };

    let mut warnings = Vec::new();
    match (expected_cache_key, cached_inputs.cache_key.as_deref()) {
        (Some(expected), Some(found)) => {
            if found != expected {
                return Ok(CacheReadResult {
                    cached_inputs: None,
                    warnings: vec![BehaviorWarning {
                        severity: WarningSeverity::Caution,
                        code: "context_cache_key_mismatch".to_owned(),
                        message: format!(
                            "Context cache at `{}` does not match the request provenance key and was bypassed.",
                            path.display()
                        ),
                    }],
                });
            }
        }
        (Some(_), None) => {
            return Ok(CacheReadResult {
                cached_inputs: None,
                warnings: vec![BehaviorWarning {
                    severity: WarningSeverity::Caution,
                    code: "context_cache_unkeyed".to_owned(),
                    message: format!(
                        "Context cache at `{}` is unkeyed and was bypassed for safety.",
                        path.display()
                    ),
                }],
            });
        }
        (None, Some(_)) => {}
        (None, None) => warnings.push(BehaviorWarning {
            severity: WarningSeverity::Info,
            code: "context_cache_unkeyed".to_owned(),
            message: format!(
                "Context cache at `{}` is unkeyed; treat it as legacy disposable data.",
                path.display()
            ),
        }),
    }

    Ok(CacheReadResult {
        cached_inputs: Some(cached_inputs),
        warnings,
    })
}

pub fn write_cached_inputs_path(
    path: impl AsRef<Path>,
    cached_inputs: &CachedInputs,
) -> Result<(), SoulError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| SoulError::Storage(error.to_string()))?;
    }

    let bytes = serde_json::to_vec_pretty(cached_inputs)
        .map_err(|error| SoulError::Storage(error.to_string()))?;

    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, &bytes).map_err(|error| SoulError::Storage(error.to_string()))?;
    fs::rename(&temp_path, path).map_err(|error| SoulError::Storage(error.to_string()))?;

    Ok(())
}

fn write_cached_inputs_path_with_key(
    path: impl AsRef<Path>,
    cached_inputs: &CachedInputs,
    cache_key: &str,
) -> Result<(), SoulError> {
    let mut payload = cached_inputs.clone();
    payload.cache_key = Some(cache_key.to_owned());
    write_cached_inputs_path(path, &payload)
}

#[cfg(test)]
mod tests {
    use crate::domain::ComposeRequest;

    use super::context_cache_key;

    #[test]
    fn context_cache_key_is_stable_for_same_request_provenance() {
        let request = ComposeRequest::new("alpha", "session-1");

        let first = context_cache_key(&request);
        let second = context_cache_key(&request);

        assert_eq!(first, second);
    }

    #[test]
    fn context_cache_key_changes_for_different_request_provenance() {
        let mut first = ComposeRequest::new("alpha", "session-1");
        first.workspace_id = "/tmp/a".to_owned();
        first.registry_reputation_path = Some("/tmp/a/rep.json".to_owned());

        let mut second = first.clone();
        second.registry_reputation_path = Some("/tmp/a/rep-v2.json".to_owned());

        assert_ne!(context_cache_key(&first), context_cache_key(&second));
    }
}
