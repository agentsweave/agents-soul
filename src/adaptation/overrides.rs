#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverrideMaterializer;

impl OverrideMaterializer {
    pub const fn role() -> &'static str {
        "Materialize effective overrides from bounded adaptation state."
    }
}
