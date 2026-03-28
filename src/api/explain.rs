#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplainEndpoint;

impl ExplainEndpoint {
    pub const fn role() -> &'static str {
        "Expose explain output without reimplementing provenance rules."
    }
}
