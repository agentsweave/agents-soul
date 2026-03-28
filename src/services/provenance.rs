use serde::Serialize;

use crate::{
    app::hash::stable_hash,
    domain::{NormalizedInputs, ProvenanceReport, SessionIdentitySnapshot},
};

#[derive(Debug, Clone, Default)]
pub struct ProvenanceService;

impl ProvenanceService {
    pub fn build(&self, normalized: &NormalizedInputs) -> ProvenanceReport {
        ProvenanceReport {
            identity_fingerprint: normalized
                .identity_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.fingerprint.clone())
                .or_else(|| {
                    normalized
                        .identity_snapshot
                        .as_ref()
                        .map(identity_fingerprint)
                }),
            registry_verification_at: normalized
                .verification_result
                .as_ref()
                .and_then(|verification| verification.verified_at),
            config_hash: prefixed_hash("cfg", &normalized.soul_config),
            adaptation_hash: prefixed_hash("adp", &normalized.adaptation_state),
            input_hash: prefixed_hash("inp", normalized),
        }
    }
}

fn identity_fingerprint(snapshot: &SessionIdentitySnapshot) -> String {
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

fn prefixed_hash<T: Serialize>(prefix: &str, value: &T) -> String {
    let payload = serde_json::to_string(value).unwrap_or_default();
    format!("{prefix}_{:016x}", stable_hash(payload))
}
