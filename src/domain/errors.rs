use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoulError {
    InvalidRequest(&'static str),
    Unavailable(&'static str),
}

impl fmt::Display for SoulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(f, "invalid request: {message}"),
            Self::Unavailable(message) => write!(f, "unavailable: {message}"),
        }
    }
}

impl Error for SoulError {}
