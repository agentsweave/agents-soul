use crate::{
    domain::{ComposeRequest, SoulError},
    services::SoulServices,
};

pub fn render(services: &SoulServices) -> Result<String, SoulError> {
    let context = services
        .compose
        .compose(ComposeRequest::new("agent.bootstrap", "session.bootstrap"))?;

    Ok(format!(
        "mode={:?} profile={} warnings={}",
        context.mode,
        context.profile_name,
        context.warnings.join("; ")
    ))
}
