#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunicationStyle {
    pub tone: String,
    pub directness: u8,
}

impl Default for CommunicationStyle {
    fn default() -> Self {
        Self {
            tone: "grounded".to_string(),
            directness: 50,
        }
    }
}
