use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionEvent {
    pub event_id: String,
    pub agent_id: String,
    pub signal_kind: String,
    pub signal_value: f32,
    pub context_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
