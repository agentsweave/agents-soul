pub mod adaptation;
pub mod behavioral_context;
pub mod compose;
pub mod config;
pub mod context;
pub mod errors;
pub mod heuristics;
pub mod inputs;
pub mod interactions;
pub mod limits;
pub mod profile;
pub mod provenance;
pub mod status;
pub mod style;

pub use adaptation::{
    AdaptationState, CommunicationOverride, HeuristicOverride, PersonalityOverride,
};
pub use behavioral_context::{BehaviorWarning, BehavioralContext, WarningSeverity};
pub use config::{
    AdaptationConfig, CURRENT_SCHEMA_VERSION, SoulConfig, SourceConfig, TemplateConfig,
};
pub use errors::{SoulError, SoulErrorCategory, SoulTransportError};
pub use heuristics::DecisionHeuristic;
pub use inputs::{
    BehaviorInputs, ComposeRequest, InputProvenance, InputSourceKind, NormalizedInputs,
    RelationshipMarker, ReputationSummary, SessionIdentitySnapshot, VerificationResult,
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
