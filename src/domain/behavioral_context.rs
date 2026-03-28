use super::{CURRENT_SCHEMA_VERSION, PersonalityProfile, ProvenanceReport, StatusSummary};

#[derive(Debug, Clone, PartialEq)]
pub struct BehavioralContext {
    pub schema_version: u32,
    pub agent_id: String,
    pub profile_name: String,
    pub status_summary: StatusSummary,
    pub trait_profile: PersonalityProfile,
    pub communication_rules: Vec<String>,
    pub decision_rules: Vec<String>,
    pub active_commitments: Vec<String>,
    pub relationship_context: Vec<String>,
    pub adaptive_notes: Vec<String>,
    pub warnings: Vec<String>,
    pub system_prompt_prefix: String,
    pub provenance: ProvenanceReport,
}

impl Default for BehavioralContext {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            agent_id: String::new(),
            profile_name: String::new(),
            status_summary: StatusSummary::default(),
            trait_profile: PersonalityProfile::default(),
            communication_rules: Vec::new(),
            decision_rules: Vec::new(),
            active_commitments: Vec::new(),
            relationship_context: Vec::new(),
            adaptive_notes: Vec::new(),
            warnings: Vec::new(),
            system_prompt_prefix: String::new(),
            provenance: ProvenanceReport::default(),
        }
    }
}
