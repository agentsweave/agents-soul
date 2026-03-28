#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplatesService;

impl TemplatesService {
    pub const fn role() -> &'static str {
        "Render stable text and JSON templates from shared context."
    }
}
