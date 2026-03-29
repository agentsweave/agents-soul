//! CLI compose command surface.

use crate::{
    app::errors::{SoulTransportError, map_soul_error},
    domain::SoulError,
};

pub fn map_compose_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}
