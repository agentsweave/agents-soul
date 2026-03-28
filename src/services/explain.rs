#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplainService;

impl ExplainService {
    pub const fn role() -> &'static str {
        "Produce explain and inspect views over the shared domain state."
    }
}
