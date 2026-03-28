use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProvenanceReport {
    pub identity_fingerprint: Option<String>,
    pub registry_verification_at: Option<DateTime<Utc>>,
    pub config_hash: String,
    pub adaptation_hash: String,
    pub input_hash: String,
}
