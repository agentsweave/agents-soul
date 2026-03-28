#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WarningsService;

impl WarningsService {
    pub const fn role() -> &'static str {
        "Generate operator-facing warnings from composition state."
    }
}
