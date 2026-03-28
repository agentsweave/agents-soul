#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionsEndpoint;

impl InteractionsEndpoint {
    pub const fn role() -> &'static str {
        "Record interactions while leaving adaptation logic in core services."
    }
}
