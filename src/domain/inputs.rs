use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AdaptationState, BehaviorWarning, CURRENT_SCHEMA_VERSION, ComposeMode, RecoveryState,
    RegistryStatus, SoulConfig, SoulError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeRequest {
    pub workspace_id: String,
    pub agent_id: String,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_snapshot_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_verification_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_reputation_path: Option<String>,
    #[serde(default = "default_true")]
    pub include_reputation: bool,
    #[serde(default = "default_true")]
    pub include_relationships: bool,
    #[serde(default = "default_true")]
    pub include_commitments: bool,
}

impl ComposeRequest {
    pub fn new(agent_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        let agent_id = agent_id.into();
        Self {
            workspace_id: ".".to_owned(),
            agent_id,
            session_id: session_id.into(),
            identity_snapshot_path: None,
            registry_verification_path: None,
            registry_reputation_path: None,
            include_reputation: true,
            include_relationships: true,
            include_commitments: true,
        }
    }

    pub fn validate(&self) -> Result<(), SoulError> {
        if self.workspace_id.trim().is_empty() {
            return Err(SoulError::EmptyField("workspace_id"));
        }
        if self.agent_id.trim().is_empty() {
            return Err(SoulError::EmptyField("agent_id"));
        }
        if self.session_id.trim().is_empty() {
            return Err(SoulError::EmptyField("session_id"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InputSourceKind {
    Explicit,
    Live,
    Cache,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InputProvenance {
    pub source: InputSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl InputProvenance {
    pub fn explicit(detail: impl Into<String>) -> Self {
        Self {
            source: InputSourceKind::Explicit,
            detail: Some(detail.into()),
        }
    }

    pub fn live(detail: impl Into<String>) -> Self {
        Self {
            source: InputSourceKind::Live,
            detail: Some(detail.into()),
        }
    }

    pub fn cache(detail: impl Into<String>) -> Self {
        Self {
            source: InputSourceKind::Cache,
            detail: Some(detail.into()),
        }
    }

    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self {
            source: InputSourceKind::Unavailable,
            detail: Some(detail.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipMarker {
    pub subject: String,
    pub marker: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionIdentitySnapshot {
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub recovery_state: RecoveryState,
    #[serde(default)]
    pub active_commitments: Vec<String>,
    #[serde(default)]
    pub durable_preferences: Vec<String>,
    #[serde(default)]
    pub relationship_markers: Vec<RelationshipMarker>,
    #[serde(default)]
    pub facts: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<BehaviorWarning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IdentifySignals {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<SessionIdentitySnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_state: Option<RecoveryState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryStanding {
    pub status: RegistryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
}

pub type VerificationResult = RegistryStanding;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RegistryReputation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_total: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_recent_30d: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub context: Vec<String>,
}

pub type ReputationSummary = RegistryReputation;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RegistrySnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<RegistryStanding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reputation: Option<RegistryReputation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorInputs {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_snapshot: Option<SessionIdentitySnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_recovery_state: Option<RecoveryState>,
    #[serde(default)]
    pub identity_provenance: InputProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_result: Option<VerificationResult>,
    #[serde(default)]
    pub verification_provenance: InputProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reputation_summary: Option<ReputationSummary>,
    #[serde(default)]
    pub reputation_provenance: InputProvenance,
    pub soul_config: SoulConfig,
    #[serde(default)]
    pub adaptation_state: AdaptationState,
    #[serde(default)]
    pub reader_warnings: Vec<BehaviorWarning>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedInputs {
    pub schema_version: u32,
    pub request: ComposeRequest,
    pub agent_id: String,
    pub profile_name: String,
    pub compose_mode_hint: Option<ComposeMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_snapshot: Option<SessionIdentitySnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_recovery_state: Option<RecoveryState>,
    #[serde(default)]
    pub identity_provenance: InputProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_result: Option<VerificationResult>,
    #[serde(default)]
    pub verification_provenance: InputProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reputation_summary: Option<ReputationSummary>,
    #[serde(default)]
    pub reputation_provenance: InputProvenance,
    pub soul_config: SoulConfig,
    pub adaptation_state: AdaptationState,
    #[serde(default)]
    pub reader_warnings: Vec<BehaviorWarning>,
    pub generated_at: DateTime<Utc>,
}

impl Default for BehaviorInputs {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            identity_snapshot: None,
            identity_recovery_state: None,
            identity_provenance: InputProvenance::unavailable("identity not requested"),
            verification_result: None,
            verification_provenance: InputProvenance::unavailable(
                "registry verification not requested",
            ),
            reputation_summary: None,
            reputation_provenance: InputProvenance::unavailable(
                "registry reputation not requested",
            ),
            soul_config: SoulConfig::default(),
            adaptation_state: AdaptationState::default(),
            reader_warnings: Vec::new(),
            generated_at: Utc::now(),
        }
    }
}

fn default_true() -> bool {
    true
}
