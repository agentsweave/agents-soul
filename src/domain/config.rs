use crate::domain::SoulLimits;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulConfig {
    pub profile_name: String,
    pub limits: SoulLimits,
}

impl Default for SoulConfig {
    fn default() -> Self {
        Self {
            profile_name: "baseline".to_string(),
            limits: SoulLimits::default(),
        }
    }
}
