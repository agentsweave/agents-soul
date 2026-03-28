#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixtureCatalog;

impl FixtureCatalog {
    pub const fn role() -> &'static str {
        "Provide deterministic fixture surfaces for tests and snapshots."
    }
}
