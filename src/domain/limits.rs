use serde::{Deserialize, Serialize};

use super::SoulError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OfflineRegistryBehavior {
    #[default]
    Cautious,
    BaselineOnly,
    FailClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RevokedBehavior {
    #[default]
    FailClosed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoulLimits {
    pub max_trait_drift: f32,
    pub max_prompt_prefix_chars: usize,
    pub max_adaptive_rules: usize,
    #[serde(default)]
    pub offline_registry_behavior: OfflineRegistryBehavior,
    #[serde(default)]
    pub revoked_behavior: RevokedBehavior,
}

impl SoulLimits {
    pub fn validate(&self) -> Result<(), SoulError> {
        if !(0.0..=1.0).contains(&self.max_trait_drift) {
            return Err(SoulError::InvalidConfig(format!(
                "max_trait_drift must be within 0.0..=1.0, got {}",
                self.max_trait_drift
            )));
        }
        if self.max_prompt_prefix_chars == 0 {
            return Err(SoulError::InvalidConfig(
                "max_prompt_prefix_chars must be greater than zero".into(),
            ));
        }
        if self.max_adaptive_rules == 0 {
            return Err(SoulError::InvalidConfig(
                "max_adaptive_rules must be greater than zero".into(),
            ));
        }
        Ok(())
    }
}

impl Default for SoulLimits {
    fn default() -> Self {
        Self {
            max_trait_drift: 0.15,
            max_prompt_prefix_chars: 4_000,
            max_adaptive_rules: 24,
            offline_registry_behavior: OfflineRegistryBehavior::Cautious,
            revoked_behavior: RevokedBehavior::FailClosed,
        }
    }
}
