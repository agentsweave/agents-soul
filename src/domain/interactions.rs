#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionEvent {
    pub interaction_type: String,
    pub outcome: String,
}
