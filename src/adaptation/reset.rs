#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdaptationReset;

impl AdaptationReset {
    pub const fn role() -> &'static str {
        "Reset adaptation state without rewriting baseline profile data."
    }
}
