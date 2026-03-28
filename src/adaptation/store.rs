#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdaptationStore;

impl AdaptationStore {
    pub const fn role() -> &'static str {
        "Persist adaptive state separately from baseline configuration."
    }
}
