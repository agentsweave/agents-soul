use serde::Serialize;

use crate::{
    app::hash::stable_hash,
    domain::{
        AdaptationState, NormalizedInputs, ProvenanceReport, SessionIdentitySnapshot, SoulConfig,
        SoulError,
    },
};

pub trait ProvenanceHasher: Send + Sync {
    fn identity_fingerprint(&self, snapshot: &SessionIdentitySnapshot)
    -> Result<String, SoulError>;
    fn config_hash(&self, config: &SoulConfig) -> Result<String, SoulError>;
    fn adaptation_hash(&self, state: &AdaptationState) -> Result<String, SoulError>;
    fn input_hash(&self, normalized: &NormalizedInputs) -> Result<String, SoulError>;
}

#[derive(Debug, Clone, Default)]
pub struct StableProvenanceHasher;

impl ProvenanceHasher for StableProvenanceHasher {
    fn identity_fingerprint(
        &self,
        snapshot: &SessionIdentitySnapshot,
    ) -> Result<String, SoulError> {
        prefixed_hash(
            "id",
            &(
                &snapshot.agent_id,
                &snapshot.display_name,
                snapshot.recovery_state,
                &snapshot.active_commitments,
                &snapshot.durable_preferences,
                &snapshot.relationship_markers,
                &snapshot.facts,
            ),
        )
    }

    fn config_hash(&self, config: &SoulConfig) -> Result<String, SoulError> {
        prefixed_hash("cfg", config)
    }

    fn adaptation_hash(&self, state: &AdaptationState) -> Result<String, SoulError> {
        prefixed_hash("adp", state)
    }

    fn input_hash(&self, normalized: &NormalizedInputs) -> Result<String, SoulError> {
        prefixed_hash("inp", normalized)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProvenanceService;

impl ProvenanceService {
    pub fn build(
        &self,
        hashing: &dyn ProvenanceHasher,
        normalized: &NormalizedInputs,
    ) -> Result<ProvenanceReport, SoulError> {
        let identity_fingerprint = match normalized.upstream.identity.snapshot.as_ref() {
            Some(snapshot) => match snapshot.fingerprint.clone() {
                Some(fingerprint) => Some(fingerprint),
                None => Some(hashing.identity_fingerprint(snapshot)?),
            },
            None => None,
        };

        Ok(ProvenanceReport {
            identity_fingerprint,
            registry_verification_at: normalized
                .upstream
                .registry
                .verification
                .as_ref()
                .and_then(|verification| verification.verified_at),
            identity_source: normalized.upstream.identity.provenance.source,
            verification_source: normalized.upstream.registry.verification_provenance.source,
            reputation_source: normalized.upstream.registry.reputation_provenance.source,
            config_hash: hashing.config_hash(&normalized.soul_config)?,
            adaptation_hash: hashing.adaptation_hash(&normalized.adaptation_state)?,
            input_hash: hashing.input_hash(normalized)?,
        })
    }
}

fn prefixed_hash<T: Serialize>(prefix: &str, value: &T) -> Result<String, SoulError> {
    let payload = serde_json::to_string(value).map_err(|error| {
        SoulError::Internal(format!("failed to serialize provenance input: {error}"))
    })?;
    Ok(format!("{prefix}_{:016x}", stable_hash(payload)))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::domain::{
        AdaptationState, BehaviorInputs, ComposeRequest, InputProvenance, InputSourceKind,
        RecoveryState, RegistryStatus, SessionIdentitySnapshot, SoulConfig, VerificationResult,
    };
    use crate::sources::normalize::normalize_inputs;

    use super::{ProvenanceService, StableProvenanceHasher};

    #[test]
    fn build_preserves_upstream_sources_and_timestamp() {
        let request = ComposeRequest::new("alpha", "session-1");
        let verified_at = Utc
            .with_ymd_and_hms(2026, 3, 29, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: SoulConfig {
                    agent_id: "alpha".into(),
                    profile_name: "Alpha".into(),
                    ..SoulConfig::default()
                },
                identity_snapshot: Some(SessionIdentitySnapshot {
                    agent_id: "alpha".into(),
                    display_name: Some("Alpha".into()),
                    recovery_state: RecoveryState::Healthy,
                    active_commitments: vec!["protect operator".into()],
                    durable_preferences: vec!["be concise".into()],
                    relationship_markers: Vec::new(),
                    facts: vec!["fact-a".into()],
                    warnings: Vec::new(),
                    fingerprint: None,
                }),
                identity_provenance: InputProvenance::live("session_identity_snapshot.json"),
                verification_result: Some(VerificationResult {
                    status: RegistryStatus::Active,
                    standing_level: Some("good".into()),
                    reason_code: None,
                    verified_at: Some(verified_at),
                }),
                verification_provenance: InputProvenance::explicit("registry_verification.json"),
                reputation_summary: Some(crate::domain::ReputationSummary {
                    score_total: Some(4.9),
                    score_recent_30d: Some(4.7),
                    last_event_at: Some(verified_at),
                    context: vec!["recent-good".into()],
                }),
                reputation_provenance: InputProvenance::cache("context_cache.json"),
                generated_at: verified_at,
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let report = ProvenanceService
            .build(&StableProvenanceHasher, &normalized)
            .expect("provenance report");

        assert_eq!(report.registry_verification_at, Some(verified_at));
        assert_eq!(report.identity_source, InputSourceKind::Live);
        assert_eq!(report.verification_source, InputSourceKind::Explicit);
        assert_eq!(report.reputation_source, InputSourceKind::Cache);
        assert!(report.identity_fingerprint.is_some());
        assert!(report.config_hash.starts_with("cfg_"));
        assert!(report.adaptation_hash.starts_with("adp_"));
        assert!(report.input_hash.starts_with("inp_"));
    }

    #[test]
    fn build_reuses_existing_identity_fingerprint_and_stable_hashes() {
        let request = ComposeRequest::new("alpha", "session-1");
        let generated_at = Utc
            .with_ymd_and_hms(2026, 3, 29, 12, 30, 0)
            .single()
            .expect("valid timestamp");

        let build_inputs = || {
            normalize_inputs(
                &request,
                BehaviorInputs {
                    soul_config: SoulConfig {
                        agent_id: "alpha".into(),
                        profile_name: "Alpha".into(),
                        ..SoulConfig::default()
                    },
                    identity_snapshot: Some(SessionIdentitySnapshot {
                        agent_id: "alpha".into(),
                        display_name: Some("Alpha".into()),
                        recovery_state: RecoveryState::Healthy,
                        active_commitments: vec!["commit-a".into()],
                        durable_preferences: vec!["pref-a".into()],
                        relationship_markers: Vec::new(),
                        facts: vec!["fact-a".into()],
                        warnings: Vec::new(),
                        fingerprint: Some("fingerprint-from-identify".into()),
                    }),
                    identity_provenance: InputProvenance::live("session_identity_snapshot.json"),
                    verification_result: Some(VerificationResult {
                        status: RegistryStatus::Active,
                        standing_level: Some("good".into()),
                        reason_code: None,
                        verified_at: None,
                    }),
                    verification_provenance: InputProvenance::live("registry_verification.json"),
                    reputation_provenance: InputProvenance::unavailable("not requested"),
                    adaptation_state: AdaptationState::default(),
                    generated_at,
                    ..BehaviorInputs::default()
                },
            )
            .expect("normalized inputs")
        };

        let first = ProvenanceService
            .build(&StableProvenanceHasher, &build_inputs())
            .expect("first provenance report");
        let second = ProvenanceService
            .build(&StableProvenanceHasher, &build_inputs())
            .expect("second provenance report");

        assert_eq!(
            first.identity_fingerprint.as_deref(),
            Some("fingerprint-from-identify")
        );
        assert_eq!(first.config_hash, second.config_hash);
        assert_eq!(first.adaptation_hash, second.adaptation_hash);
        assert_eq!(first.input_hash, second.input_hash);
    }
}
