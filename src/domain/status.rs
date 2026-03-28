#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryStanding {
    Active,
    Suspended,
    Revoked,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSummary {
    pub registry: RegistryStanding,
    pub degraded: bool,
    pub notes: Vec<String>,
}

impl StatusSummary {
    pub fn baseline_only() -> Self {
        Self {
            registry: RegistryStanding::Unavailable,
            degraded: true,
            notes: vec!["upstream readers are not wired yet".to_string()],
        }
    }
}
