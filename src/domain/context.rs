use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{CURRENT_SCHEMA_VERSION, ComposeMode, PersonalityProfile};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehavioralContext {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub agent_id: String,
    pub profile_name: String,
    pub status_summary: StatusSummary,
    pub trait_profile: PersonalityProfile,
    #[serde(default)]
    pub communication_rules: Vec<String>,
    #[serde(default)]
    pub decision_rules: Vec<String>,
    #[serde(default)]
    pub active_commitments: Vec<String>,
    #[serde(default)]
    pub relationship_context: Vec<String>,
    #[serde(default)]
    pub adaptive_notes: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<BehavioralWarning>,
    pub system_prompt_prefix: String,
    pub provenance: ProvenanceReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusSummary {
    pub compose_mode: ComposeMode,
    pub identity_loaded: bool,
    pub registry_verified: bool,
    #[serde(default)]
    pub registry_status: Option<RegistryStatus>,
    pub reputation_loaded: bool,
    #[serde(default)]
    pub recovery_state: Option<RecoveryState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceReport {
    #[serde(default)]
    pub identity_fingerprint: Option<String>,
    #[serde(default)]
    pub registry_verification_at: Option<DateTime<Utc>>,
    pub config_hash: String,
    pub adaptation_hash: String,
    pub input_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BehavioralWarning {
    pub severity: WarningSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryStatus {
    Active,
    Pending,
    Suspended,
    Revoked,
    Retired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryState {
    Healthy,
    Degraded,
    Broken,
}

const fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}
