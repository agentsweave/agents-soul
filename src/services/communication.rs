#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommunicationService;

impl CommunicationService {
    pub const fn role() -> &'static str {
        "Derive communication style from domain inputs."
    }
}
