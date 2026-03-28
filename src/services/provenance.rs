#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProvenanceService;

impl ProvenanceService {
    pub const fn role() -> &'static str {
        "Track where each piece of behavior came from."
    }
}
