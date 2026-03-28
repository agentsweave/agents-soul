#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmaReducer;

impl EmaReducer {
    pub const fn role() -> &'static str {
        "Reduce interaction history into bounded adaptation signals."
    }
}
