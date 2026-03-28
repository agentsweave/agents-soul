use crate::domain::{
    CommunicationStyle, ComposeRequest, DecisionHeuristic, ProvenanceReport, StatusSummary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeMode {
    Normal,
    Restricted,
    Degraded,
    BaselineOnly,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehavioralContext {
    pub request: ComposeRequest,
    pub mode: ComposeMode,
    pub profile_name: String,
    pub style: CommunicationStyle,
    pub heuristics: Vec<DecisionHeuristic>,
    pub warnings: Vec<String>,
    pub status: StatusSummary,
    pub provenance: ProvenanceReport,
}

impl BehavioralContext {
    pub fn skeleton(request: ComposeRequest) -> Self {
        Self {
            request,
            mode: ComposeMode::BaselineOnly,
            profile_name: "baseline".to_string(),
            style: CommunicationStyle::default(),
            heuristics: vec![DecisionHeuristic {
                name: "transport-parity".to_string(),
                summary: "All transports must delegate to shared services.".to_string(),
            }],
            warnings: vec!["crate layout bootstrapped; compose logic not implemented".to_string()],
            status: StatusSummary::baseline_only(),
            provenance: ProvenanceReport::bootstrap(),
        }
    }
}
