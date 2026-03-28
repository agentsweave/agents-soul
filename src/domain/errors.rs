use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SoulError {
    #[error("failed to read soul config at `{path}`: {message}")]
    ConfigRead { path: String, message: String },
    #[error("failed to parse soul config at `{path}`: {message}")]
    ConfigParse { path: String, message: String },
    #[error("invalid soul config: {0}")]
    InvalidConfig(String),
    #[error("field `{field}` must be within 0.0..=1.0, got {value}")]
    InvalidTraitValue { field: &'static str, value: f32 },
    #[error("field `{0}` must not be empty")]
    EmptyField(&'static str),
    #[error("duplicate heuristic id `{0}`")]
    DuplicateHeuristicId(String),
    #[error("required upstream inputs are broken")]
    RequiredInputsBroken,
    #[error("identity input unavailable")]
    IdentityUnavailable,
    #[error("registry verification unavailable")]
    RegistryUnavailable,
    #[error("revoked identity cannot compose normal context")]
    RevokedIdentity,
    #[error("storage error: {0}")]
    Storage(String),
}
