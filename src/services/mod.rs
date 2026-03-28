pub mod commitments;
pub mod communication;
pub mod compose;
pub mod decision_rules;
pub mod explain;
pub mod limits;
pub mod profile;
pub mod provenance;
pub mod relationships;
pub mod templates;
pub mod warnings;

use crate::services::{
    commitments::CommitmentsService, communication::CommunicationService, compose::ComposeService,
    decision_rules::DecisionRulesService, explain::ExplainService, limits::LimitsService,
    profile::ProfileService, provenance::ProvenanceService, relationships::RelationshipsService,
    templates::TemplatesService, warnings::WarningsService,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoulServices {
    pub compose: ComposeService,
    pub profile: ProfileService,
    pub communication: CommunicationService,
    pub decision_rules: DecisionRulesService,
    pub relationships: RelationshipsService,
    pub commitments: CommitmentsService,
    pub warnings: WarningsService,
    pub provenance: ProvenanceService,
    pub explain: ExplainService,
    pub templates: TemplatesService,
    pub limits: LimitsService,
}
