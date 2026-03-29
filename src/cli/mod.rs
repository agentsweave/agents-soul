pub mod compose;
pub mod configure;
pub mod explain;
pub mod inspect;
pub mod reset;

use crate::{
    app::{config::ApplicationConfig, deps::AppDeps},
    domain::SoulError,
};

pub fn run(_config: &ApplicationConfig, _deps: &AppDeps) -> Result<(), SoulError> {
    Ok(())
}
