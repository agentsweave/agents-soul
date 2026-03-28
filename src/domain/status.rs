use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ComposeMode {
    Normal,
    Restricted,
    Degraded,
    #[default]
    BaselineOnly,
    FailClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryStatus {
    Active,
    Pending,
    Suspended,
    Revoked,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryState {
    Healthy,
    Recovering,
    Degraded,
    Broken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StatusSummary {
    pub compose_mode: ComposeMode,
    pub identity_loaded: bool,
    pub registry_verified: bool,
    pub registry_status: Option<RegistryStatus>,
    pub reputation_loaded: bool,
    pub recovery_state: Option<RecoveryState>,
}
