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
pub mod workspace_config;

pub use commitments::CommitmentsService;
pub use communication::CommunicationRulesService;
pub use compose::ComposeService;
pub use decision_rules::DecisionRulesService;
pub use profile::EffectiveProfileService;
pub use provenance::ProvenanceService;
pub use relationships::RelationshipsService;
pub use warnings::WarningService;
pub use workspace_config::WorkspaceConfigService;

pub type ServiceError = crate::domain::SoulError;

#[derive(Debug, Clone, Default)]
pub struct SoulServices {
    pub compose: ComposeService,
    pub workspace_config: WorkspaceConfigService,
}
