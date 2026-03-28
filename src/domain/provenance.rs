use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::InputSourceKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProvenanceReport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_verification_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub identity_source: InputSourceKind,
    #[serde(default)]
    pub verification_source: InputSourceKind,
    #[serde(default)]
    pub reputation_source: InputSourceKind,
    pub config_hash: String,
    pub adaptation_hash: String,
    pub input_hash: String,
}
