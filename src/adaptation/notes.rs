#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdaptiveNotes;

impl AdaptiveNotes {
    pub const fn role() -> &'static str {
        "Render bounded adaptive notes for inspect and explain surfaces."
    }
}
