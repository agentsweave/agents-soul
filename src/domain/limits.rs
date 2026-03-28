#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulLimits {
    pub max_trait_drift: u8,
    pub max_adaptive_notes: u8,
}

impl Default for SoulLimits {
    fn default() -> Self {
        Self {
            max_trait_drift: 20,
            max_adaptive_notes: 8,
        }
    }
}
