#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeuristicsEndpoint;

impl HeuristicsEndpoint {
    pub const fn role() -> &'static str {
        "Expose heuristics without duplicating decision logic."
    }
}
