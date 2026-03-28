mod adaptation;
mod behavioral_context;
mod config;
mod errors;
mod heuristics;
mod inputs;
mod interactions;
mod limits;
mod profile;
mod provenance;
mod status;
mod style;

pub use adaptation::{
    AdaptationState, CommunicationOverride, HeuristicOverride, PersonalityOverride,
};
pub use behavioral_context::{BehaviorWarning, BehavioralContext, WarningSeverity};
pub use config::{
    AdaptationConfig, CURRENT_SCHEMA_VERSION, SoulConfig, SourceConfig, TemplateConfig,
};
pub use errors::SoulError;
pub use heuristics::DecisionHeuristic;
pub use inputs::{
    BehaviorInputs, ComposeRequest, NormalizedInputs, RelationshipMarker, ReputationSummary,
    SessionIdentitySnapshot, VerificationResult,
};
pub use interactions::InteractionEvent;
pub use limits::{OfflineRegistryBehavior, RevokedBehavior, SoulLimits};
pub use profile::PersonalityProfile;
pub use provenance::ProvenanceReport;
pub use status::{ComposeMode, RecoveryState, RegistryStatus, StatusSummary};
pub use style::{
    CommunicationStyle, ConflictStyle, FeedbackStyle, ParagraphBudget, QuestionStyle,
    RegisterStyle, UncertaintyStyle,
};
