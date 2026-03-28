#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceCache;

impl SourceCache {
    pub const fn role() -> &'static str {
        "Cache source reads opportunistically; never become authority."
    }
}
