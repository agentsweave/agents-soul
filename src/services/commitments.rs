#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitmentsService;

impl CommitmentsService {
    pub const fn role() -> &'static str {
        "Track commitments that the compose path may surface."
    }
}
