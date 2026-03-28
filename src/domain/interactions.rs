#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionEvent {
    pub agent_id: String,
    pub interaction_type: String,
    pub outcome: String,
    pub notes: Option<String>,
}
