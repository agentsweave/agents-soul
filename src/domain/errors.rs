#[derive(Debug, Clone, PartialEq)]
pub enum SoulError {
    ConfigRead { path: String, message: String },
    ConfigParse { path: String, message: String },
    InvalidConfig(String),
    RequiredInputsBroken,
    IdentityUnavailable,
    RegistryUnavailable,
    RevokedIdentity,
    Storage(String),
    BootstrapPlaceholder(&'static str),
}

impl std::fmt::Display for SoulError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigRead { path, message } => {
                write!(f, "failed to read soul config at `{path}`: {message}")
            }
            Self::ConfigParse { path, message } => {
                write!(f, "failed to parse soul config at `{path}`: {message}")
            }
            Self::InvalidConfig(reason) => write!(f, "invalid soul config: {reason}"),
            Self::RequiredInputsBroken => write!(f, "required upstream inputs are broken"),
            Self::IdentityUnavailable => write!(f, "identity input unavailable"),
            Self::RegistryUnavailable => write!(f, "registry verification unavailable"),
            Self::RevokedIdentity => write!(f, "revoked identity cannot compose normal context"),
            Self::Storage(reason) => write!(f, "storage error: {reason}"),
            Self::BootstrapPlaceholder(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for SoulError {}
