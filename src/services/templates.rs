use crate::domain::{ComposeMode, SoulError};

pub trait PromptTemplateRenderer: Send + Sync {
    fn render_prompt_prefix(
        &self,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> Result<String, SoulError>;
}

#[derive(Debug, Clone, Default)]
pub struct TemplateService;

impl PromptTemplateRenderer for TemplateService {
    fn render_prompt_prefix(
        &self,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> Result<String, SoulError> {
        let prefix = match compose_mode {
            ComposeMode::FailClosed => [
                "Identity revoked. Do not continue normal autonomous operation.",
                "Do not present yourself as an active verified agent.",
                "State the problem plainly.",
                "Ask for operator intervention.",
                "Do not take on new commitments.",
                "Do not claim registry validity.",
            ]
            .join("\n"),
            ComposeMode::Restricted => [
                "Identity suspended. Operate in restricted advisory mode only.",
                "Lower initiative.",
                "Avoid high-risk actions.",
                "Surface uncertainty clearly.",
                "Request operator confirmation before consequential changes.",
            ]
            .join("\n"),
            ComposeMode::Degraded => {
                "Operate cautiously. Upstream identity or registry inputs are degraded, so autonomy and confidence must be reduced."
                    .to_owned()
            }
            ComposeMode::BaselineOnly => format!(
                "Use the baseline soul profile for {profile_name}. Do not invent identity-derived commitments or relationship context that was not loaded."
            ),
            ComposeMode::Normal => {
                format!("You are {profile_name}. Follow the configured soul profile.")
            }
        };

        Ok(truncate(prefix, max_chars))
    }
}

fn truncate(mut value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    value = value.chars().take(max_chars).collect();
    value.trim_end().to_owned()
}
