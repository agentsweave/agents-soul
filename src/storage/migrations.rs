#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationPlan;

impl MigrationPlan {
    pub const fn role() -> &'static str {
        "Define schema evolution without leaking storage into services."
    }
}
