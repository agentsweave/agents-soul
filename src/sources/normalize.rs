#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NormalizationPipeline;

impl NormalizationPipeline {
    pub const fn role() -> &'static str {
        "Normalize upstream inputs into deterministic reader output."
    }
}
