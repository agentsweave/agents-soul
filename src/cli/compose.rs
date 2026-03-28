//! CLI compose command surface.

use crate::domain::{SoulError, SoulTransportError};

pub fn map_compose_error(error: &SoulError) -> SoulTransportError {
    error.transport_error()
}
