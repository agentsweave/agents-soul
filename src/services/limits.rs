#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LimitsService;

impl LimitsService {
    pub const fn role() -> &'static str {
        "Enforce hard behavioral bounds before transports render output."
    }
}
