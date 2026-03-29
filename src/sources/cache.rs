use std::{fs, path::Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    app::{config::WorkspacePaths, hash::stable_hash},
    domain::{
        BehaviorWarning, ComposeRequest, ReputationSummary, SessionIdentitySnapshot, SoulError,
        VerificationResult, WarningSeverity,
    },
    services::provenance::{ProvenanceHasher, StableProvenanceHasher},
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CachedFreshness {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adaptation_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_verification_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CachedInputs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<CachedFreshness>,
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

    if let Some(reason) = stale_cached_inputs_reason(&cached_inputs)? {
        return Ok(CacheReadResult {
            cached_inputs: None,
            warnings: vec![cache_stale_warning(path, &reason)],
        });
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

pub fn cache_stale_warning(path: &Path, reason: &str) -> BehaviorWarning {
    BehaviorWarning {
        severity: WarningSeverity::Caution,
        code: "context_cache_stale".to_owned(),
        message: format!(
            "Context cache at `{}` is stale ({reason}) and was bypassed.",
            path.display()
        ),
    }
}

fn stale_cached_inputs_reason(cached_inputs: &CachedInputs) -> Result<Option<String>, SoulError> {
    let Some(freshness) = cached_inputs.freshness.as_ref() else {
        return Ok(Some("missing freshness metadata".to_owned()));
    };

    if let Some(snapshot) = cached_inputs.identity_snapshot.as_ref() {
        let fingerprint = match snapshot.fingerprint.clone() {
            Some(fingerprint) => fingerprint,
            None => StableProvenanceHasher.identity_fingerprint(snapshot)?,
        };

        match freshness.identity_fingerprint.as_deref() {
            Some(expected) if expected == fingerprint => {}
            Some(_) => return Ok(Some("identity inputs changed".to_owned())),
            None => return Ok(Some("identity freshness metadata missing".to_owned())),
        }
    }

    if cached_inputs.reputation_summary.is_some() && cached_inputs.verification_result.is_none() {
        return Ok(Some("registry freshness metadata missing".to_owned()));
    }

    if cached_inputs.verification_result.is_some() || cached_inputs.reputation_summary.is_some() {
        let cached_verified_at = cached_inputs
            .verification_result
            .as_ref()
            .and_then(|verification| verification.verified_at);
        if freshness.registry_verification_at != cached_verified_at {
            return Ok(Some("registry inputs changed".to_owned()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use crate::domain::{
        ComposeRequest, RecoveryState, RegistryStatus, SessionIdentitySnapshot, VerificationResult,
    };

    use super::{
        CachedFreshness, CachedInputs, context_cache_key, read_cached_inputs_path,
        write_cached_inputs_path,
    };

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

    #[test]
    fn read_cached_inputs_bypasses_legacy_cache_without_freshness() -> Result<(), Box<dyn Error>> {
        let path = test_cache_path("missing-freshness");
        write_cached_inputs_path(
            &path,
            &CachedInputs {
                cache_key: None,
                freshness: None,
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "alpha".to_owned(),
                    display_name: Some("Alpha".to_owned()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["cache".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: None,
                }),
                verification_result: None,
                reputation_summary: None,
            },
        )?;

        let result = read_cached_inputs_path(&path)?;
        assert!(result.cached_inputs.is_none());
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.code == "context_cache_stale")
        );

        cleanup_path(&path)?;
        Ok(())
    }

    #[test]
    fn read_cached_inputs_bypasses_identity_fingerprint_mismatch() -> Result<(), Box<dyn Error>> {
        let path = test_cache_path("identity-stale");
        write_cached_inputs_path(
            &path,
            &CachedInputs {
                cache_key: None,
                freshness: Some(CachedFreshness {
                    config_hash: Some("cfg".to_owned()),
                    adaptation_hash: Some("adp".to_owned()),
                    identity_fingerprint: Some("id_old".to_owned()),
                    registry_verification_at: None,
                }),
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "alpha".to_owned(),
                    display_name: Some("Alpha".to_owned()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["cache".to_owned()],
                    durable_preferences: Vec::new(),
                    relationship_markers: Vec::new(),
                    facts: Vec::new(),
                    warnings: Vec::new(),
                    fingerprint: Some("id_new".to_owned()),
                }),
                verification_result: None,
                reputation_summary: None,
            },
        )?;

        let result = read_cached_inputs_path(&path)?;
        assert!(result.cached_inputs.is_none());
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.code == "context_cache_stale")
        );

        cleanup_path(&path)?;
        Ok(())
    }

    #[test]
    fn read_cached_inputs_bypasses_registry_timestamp_mismatch() -> Result<(), Box<dyn Error>> {
        let path = test_cache_path("registry-stale");
        let verified_at = Utc
            .with_ymd_and_hms(2026, 3, 29, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        write_cached_inputs_path(
            &path,
            &CachedInputs {
                cache_key: None,
                freshness: Some(CachedFreshness {
                    config_hash: Some("cfg".to_owned()),
                    adaptation_hash: Some("adp".to_owned()),
                    identity_fingerprint: None,
                    registry_verification_at: Some(
                        Utc.with_ymd_and_hms(2026, 3, 28, 12, 0, 0)
                            .single()
                            .expect("valid timestamp"),
                    ),
                }),
                identity_snapshot: None,
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Active,
                    standing_level: Some("good".to_owned()),
                    reason_code: None,
                    verified_at: Some(verified_at),
                }),
                reputation_summary: None,
            },
        )?;

        let result = read_cached_inputs_path(&path)?;
        assert!(result.cached_inputs.is_none());
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.code == "context_cache_stale")
        );

        cleanup_path(&path)?;
        Ok(())
    }

    fn test_cache_path(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("agents-soul-cache-{label}-{suffix}.json"))
    }

    fn cleanup_path(path: &PathBuf) -> Result<(), Box<dyn Error>> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
