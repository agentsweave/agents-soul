#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResetEndpoint;

impl ResetEndpoint {
    pub const fn role() -> &'static str {
        "Reset adaptive state through shared reset services."
    }
}
