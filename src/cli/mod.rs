pub mod compose;
pub mod configure;
pub mod explain;
pub mod inspect;
pub mod reset;

use crate::domain::SoulError;

pub fn run() -> Result<(), SoulError> {
    Ok(())
}
