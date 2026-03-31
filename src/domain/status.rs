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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposeModeResolver;

impl ComposeModeResolver {
    pub fn resolve(
        registry_status: Option<RegistryStatus>,
        recovery_state: Option<RecoveryState>,
        offline_behavior: super::OfflineRegistryBehavior,
    ) -> ComposeMode {
        match registry_status {
            Some(RegistryStatus::Revoked) => ComposeMode::FailClosed,
            Some(RegistryStatus::Suspended) => ComposeMode::Restricted,
            Some(_) => match recovery_state {
                Some(RecoveryState::Broken)
                | Some(RecoveryState::Degraded)
                | Some(RecoveryState::Recovering) => ComposeMode::Degraded,
                Some(RecoveryState::Healthy) => ComposeMode::Normal,
                None => ComposeMode::BaselineOnly,
            },
            None => {
                if recovery_state.is_none() {
                    ComposeMode::BaselineOnly
                } else {
                    match offline_behavior {
                        super::OfflineRegistryBehavior::Cautious => ComposeMode::Degraded,
                        super::OfflineRegistryBehavior::BaselineOnly => ComposeMode::BaselineOnly,
                        super::OfflineRegistryBehavior::FailClosed => ComposeMode::FailClosed,
                    }
                }
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{ComposeMode, ComposeModeResolver, RecoveryState, RegistryStatus};
    use crate::domain::OfflineRegistryBehavior;

    #[test]
    fn resolver_fails_closed_when_revoked() {
        assert_eq!(
            ComposeModeResolver::resolve(
                Some(RegistryStatus::Revoked),
                Some(RecoveryState::Healthy),
                OfflineRegistryBehavior::Cautious,
            ),
            ComposeMode::FailClosed
        );
    }

    #[test]
    fn resolver_restricts_when_suspended() {
        assert_eq!(
            ComposeModeResolver::resolve(
                Some(RegistryStatus::Suspended),
                Some(RecoveryState::Healthy),
                OfflineRegistryBehavior::Cautious,
            ),
            ComposeMode::Restricted
        );
    }

    #[test]
    fn resolver_degrades_when_verified_identity_is_not_healthy() {
        for recovery_state in [
            RecoveryState::Broken,
            RecoveryState::Degraded,
            RecoveryState::Recovering,
        ] {
            assert_eq!(
                ComposeModeResolver::resolve(
                    Some(RegistryStatus::Active),
                    Some(recovery_state),
                    OfflineRegistryBehavior::BaselineOnly,
                ),
                ComposeMode::Degraded
            );
        }
    }

    #[test]
    fn resolver_uses_normal_when_verified_and_healthy() {
        assert_eq!(
            ComposeModeResolver::resolve(
                Some(RegistryStatus::Active),
                Some(RecoveryState::Healthy),
                OfflineRegistryBehavior::FailClosed,
            ),
            ComposeMode::Normal
        );
    }

    #[test]
    fn resolver_uses_baseline_only_when_verified_but_identity_missing() {
        assert_eq!(
            ComposeModeResolver::resolve(
                Some(RegistryStatus::Pending),
                None,
                OfflineRegistryBehavior::FailClosed,
            ),
            ComposeMode::BaselineOnly
        );
    }

    #[test]
    fn resolver_caps_offline_policy_to_baseline_only_without_identity_state() {
        assert_eq!(
            ComposeModeResolver::resolve(None, None, OfflineRegistryBehavior::FailClosed),
            ComposeMode::BaselineOnly
        );
    }

    #[test]
    fn resolver_uses_offline_policy_when_identity_state_exists_but_registry_is_unavailable() {
        assert_eq!(
            ComposeModeResolver::resolve(
                None,
                Some(RecoveryState::Recovering),
                OfflineRegistryBehavior::Cautious,
            ),
            ComposeMode::Degraded
        );
        assert_eq!(
            ComposeModeResolver::resolve(
                None,
                Some(RecoveryState::Recovering),
                OfflineRegistryBehavior::BaselineOnly,
            ),
            ComposeMode::BaselineOnly
        );
        assert_eq!(
            ComposeModeResolver::resolve(
                None,
                Some(RecoveryState::Recovering),
                OfflineRegistryBehavior::FailClosed,
            ),
            ComposeMode::FailClosed
        );
    }

    #[test]
    fn resolver_prefers_registry_terminal_states_over_identity_health() {
        let cases = [
            (
                RegistryStatus::Revoked,
                RecoveryState::Healthy,
                OfflineRegistryBehavior::BaselineOnly,
                ComposeMode::FailClosed,
            ),
            (
                RegistryStatus::Suspended,
                RecoveryState::Broken,
                OfflineRegistryBehavior::FailClosed,
                ComposeMode::Restricted,
            ),
        ];

        for (registry_status, recovery_state, offline_behavior, expected) in cases {
            assert_eq!(
                ComposeModeResolver::resolve(
                    Some(registry_status),
                    Some(recovery_state),
                    offline_behavior,
                ),
                expected
            );
        }
    }
}
