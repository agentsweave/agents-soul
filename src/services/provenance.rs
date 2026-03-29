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
        let identity_fingerprint = match normalized.identity_snapshot.as_ref() {
            Some(snapshot) => match snapshot.fingerprint.clone() {
                Some(fingerprint) => Some(fingerprint),
                None => Some(hashing.identity_fingerprint(snapshot)?),
            },
            None => None,
        };

        Ok(ProvenanceReport {
            identity_fingerprint,
            registry_verification_at: normalized
                .verification_result
                .as_ref()
                .and_then(|verification| verification.verified_at),
            identity_source: normalized.identity_provenance.source,
            verification_source: normalized.verification_provenance.source,
            reputation_source: normalized.reputation_provenance.source,
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
