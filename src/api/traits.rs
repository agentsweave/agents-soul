#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraitsEndpoint;

impl TraitsEndpoint {
    pub const fn role() -> &'static str {
        "Expose trait configuration through the shared service layer."
    }
}
