use crate::domain::{ComposeMode, SoulError};

const PROMPT_PREFIX_TEMPLATE: &str = "prompt-prefix";
const FULL_CONTEXT_TEMPLATE: &str = "full-context";
const EXPLAIN_TEMPLATE: &str = "explain";
const PROMPT_BODY_PLACEHOLDER: &str = "{{prompt_body}}";
const TITLE_PLACEHOLDER: &str = "{{title}}";
const SECTIONS_PLACEHOLDER: &str = "{{sections}}";

pub trait PromptTemplateRenderer: Send + Sync {
    fn render_prompt_prefix(
        &self,
        template_name: &str,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> Result<String, SoulError>;
}

#[derive(Debug, Clone, Default)]
pub struct TemplateService {
    engine: BuiltInTemplateEngine,
}

impl PromptTemplateRenderer for TemplateService {
    fn render_prompt_prefix(
        &self,
        template_name: &str,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> Result<String, SoulError> {
        let prompt_body = render_builtin_prompt_prefix(compose_mode, profile_name, max_chars);
        self.engine
            .render_prompt_prefix(template_name, &prompt_body)
            .map(|rendered| truncate(rendered.trim_end().to_owned(), max_chars))
    }
}

impl TemplateService {
    pub fn render_full_context(
        &self,
        template_name: &str,
        title: &str,
        sections: &[TemplateSection],
    ) -> Result<String, SoulError> {
        self.engine.render_document(template_name, title, sections)
    }

    pub fn render_explain(
        &self,
        template_name: &str,
        title: &str,
        sections: &[TemplateSection],
    ) -> Result<String, SoulError> {
        self.engine.render_document(template_name, title, sections)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateSection {
    pub heading: String,
    pub items: Vec<String>,
}

impl TemplateSection {
    pub fn new(heading: impl Into<String>, items: Vec<String>) -> Self {
        Self {
            heading: heading.into(),
            items,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct BuiltInTemplateEngine;

impl BuiltInTemplateEngine {
    fn render_prompt_prefix(
        &self,
        template_name: &str,
        prompt_body: &str,
    ) -> Result<String, SoulError> {
        let template = self.load(template_name)?;
        replace_placeholder(
            template.contents(),
            PROMPT_BODY_PLACEHOLDER,
            prompt_body,
            template.name(),
        )
    }

    fn render_document(
        &self,
        template_name: &str,
        title: &str,
        sections: &[TemplateSection],
    ) -> Result<String, SoulError> {
        let template = self.load(template_name)?;
        let rendered = replace_placeholder(
            template.contents(),
            TITLE_PLACEHOLDER,
            &render_title(title),
            template.name(),
        )?;
        let rendered = replace_placeholder(
            &rendered,
            SECTIONS_PLACEHOLDER,
            &render_sections(sections),
            template.name(),
        )?;
        Ok(rendered.trim().to_owned())
    }

    fn load(&self, template_name: &str) -> Result<BuiltInTemplate, SoulError> {
        match template_name {
            PROMPT_PREFIX_TEMPLATE => Ok(BuiltInTemplate::PromptPrefix),
            FULL_CONTEXT_TEMPLATE => Ok(BuiltInTemplate::FullContext),
            EXPLAIN_TEMPLATE => Ok(BuiltInTemplate::Explain),
            other => Err(SoulError::TemplateLoad {
                template: "built-in",
                message: format!("unknown built-in template `{other}`"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BuiltInTemplate {
    PromptPrefix,
    FullContext,
    Explain,
}

impl BuiltInTemplate {
    fn name(self) -> &'static str {
        match self {
            Self::PromptPrefix => PROMPT_PREFIX_TEMPLATE,
            Self::FullContext => FULL_CONTEXT_TEMPLATE,
            Self::Explain => EXPLAIN_TEMPLATE,
        }
    }

    fn contents(self) -> &'static str {
        match self {
            Self::PromptPrefix => include_str!("../../templates/prompt_prefix.j2"),
            Self::FullContext => include_str!("../../templates/context_full.j2"),
            Self::Explain => include_str!("../../templates/explain.j2"),
        }
    }
}

pub(crate) fn render_builtin_prompt_prefix(
    compose_mode: ComposeMode,
    profile_name: &str,
    max_chars: usize,
) -> String {
    compact_prompt_prefix(&prompt_prefix_lines(compose_mode, profile_name), max_chars)
}

fn prompt_prefix_lines(compose_mode: ComposeMode, profile_name: &str) -> Vec<String> {
    let escaped_profile_name = escape_text(profile_name);
    match compose_mode {
        ComposeMode::FailClosed => vec![
            "FAIL-CLOSED: identity revoked.".to_owned(),
            "Do not continue normal autonomous operation.".to_owned(),
            "Do not present yourself as an active verified agent.".to_owned(),
            "State the problem plainly.".to_owned(),
            "Ask for operator intervention.".to_owned(),
            "Do not take on new commitments.".to_owned(),
            "Do not claim registry validity.".to_owned(),
        ],
        ComposeMode::Restricted => vec![
            "RESTRICTED: identity suspended.".to_owned(),
            "Operate in restricted advisory mode only.".to_owned(),
            "Lower initiative.".to_owned(),
            "Avoid high-risk actions.".to_owned(),
            "Surface uncertainty clearly.".to_owned(),
            "Request operator confirmation before consequential changes.".to_owned(),
        ],
        ComposeMode::Degraded => vec![
            "DEGRADED: upstream identity or registry inputs are degraded.".to_owned(),
            "Reduce autonomy and confidence until authority is restored.".to_owned(),
        ],
        ComposeMode::BaselineOnly => vec![
            format!(
                "BASELINE-ONLY: use the baseline soul profile for {escaped_profile_name}."
            ),
            "Do not invent identity-derived commitments or relationship context that was not loaded."
                .to_owned(),
        ],
        ComposeMode::Normal => vec![
            format!("PROFILE: {escaped_profile_name}."),
            "Follow the configured soul profile.".to_owned(),
        ],
    }
}

fn compact_prompt_prefix(lines: &[String], max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let mut rendered = String::new();
    for line in lines {
        let candidate = if rendered.is_empty() {
            line.clone()
        } else {
            format!("{rendered}\n{line}")
        };

        if candidate.chars().count() <= max_chars {
            rendered = candidate;
            continue;
        }

        if rendered.is_empty() {
            return truncate(line.to_owned(), max_chars);
        }

        return rendered;
    }

    rendered
}

fn render_title(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        escape_text(trimmed)
    }
}

fn render_sections(sections: &[TemplateSection]) -> String {
    sections
        .iter()
        .filter_map(render_section)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_section(section: &TemplateSection) -> Option<String> {
    let items = section
        .items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| format!("- {}", escape_text(item)))
        .collect::<Vec<_>>();

    if items.is_empty() {
        return None;
    }

    Some(format!(
        "## {}\n{}",
        escape_text(section.heading.trim()),
        items.join("\n")
    ))
}

fn replace_placeholder(
    template: &str,
    placeholder: &str,
    value: &str,
    template_name: &'static str,
) -> Result<String, SoulError> {
    if !template.contains(placeholder) {
        return Err(SoulError::TemplateRender {
            template: template_name,
            message: format!("missing placeholder `{placeholder}`"),
        });
    }

    Ok(template.replace(placeholder, value))
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn truncate(mut value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    value = value.chars().take(max_chars).collect();
    value.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        EXPLAIN_TEMPLATE, FULL_CONTEXT_TEMPLATE, PROMPT_PREFIX_TEMPLATE, TemplateSection,
        TemplateService,
    };
    use crate::domain::{ComposeMode, SoulError};
    use crate::services::templates::PromptTemplateRenderer;

    #[test]
    fn prompt_prefix_loads_from_builtin_template() {
        let rendered = TemplateService::default()
            .render_prompt_prefix(PROMPT_PREFIX_TEMPLATE, ComposeMode::Normal, "Alpha", 512)
            .expect("prompt prefix should render");

        assert_eq!(
            rendered,
            "PROFILE: Alpha.\nFollow the configured soul profile."
        );
    }

    #[test]
    fn prompt_prefix_preserves_fail_closed_safety_cue_when_budget_is_tiny() {
        let rendered = TemplateService::default()
            .render_prompt_prefix(PROMPT_PREFIX_TEMPLATE, ComposeMode::FailClosed, "Alpha", 12)
            .expect("prompt prefix should render");

        assert_eq!(rendered, "FAIL-CLOSED:");
    }

    #[test]
    fn prompt_prefix_compacts_by_whole_lines_before_dropping_detail() {
        let first_line = "RESTRICTED: identity suspended.";
        let rendered = TemplateService::default()
            .render_prompt_prefix(
                PROMPT_PREFIX_TEMPLATE,
                ComposeMode::Restricted,
                "Alpha",
                first_line.chars().count(),
            )
            .expect("prompt prefix should render");

        assert_eq!(rendered, first_line);
        assert!(!rendered.contains('\n'));
    }

    #[test]
    fn prompt_prefix_escapes_profile_name_free_text() {
        let rendered = TemplateService::default()
            .render_prompt_prefix(
                PROMPT_PREFIX_TEMPLATE,
                ComposeMode::Normal,
                "Alpha <Builder> & \"owner\"",
                512,
            )
            .expect("prompt prefix should render");

        assert_eq!(
            rendered,
            "PROFILE: Alpha &lt;Builder&gt; &amp; &quot;owner&quot;.\nFollow the configured soul profile."
        );
    }

    #[test]
    fn document_rendering_escapes_free_text_and_omits_missing_sections() {
        let rendered = TemplateService::default()
            .render_full_context(
                FULL_CONTEXT_TEMPLATE,
                "Alpha <Builder>",
                &[
                    TemplateSection::new(
                        "Relationships",
                        vec!["trusted <operator> & shipping partner".to_owned()],
                    ),
                    TemplateSection::new("Commitments", Vec::new()),
                    TemplateSection::new(
                        "Adaptive Notes",
                        vec!["Keep \"commitments\" stable".to_owned()],
                    ),
                ],
            )
            .expect("full context should render");

        assert!(rendered.starts_with("Alpha &lt;Builder&gt;"));
        assert!(rendered.contains("## Relationships"));
        assert!(rendered.contains("trusted &lt;operator&gt; &amp; shipping partner"));
        assert!(rendered.contains("Keep &quot;commitments&quot; stable"));
        assert!(!rendered.contains("## Commitments"));
    }

    #[test]
    fn full_context_renderer_preserves_clean_section_spacing_and_escapes_items() {
        let rendered = TemplateService::default()
            .render_full_context(
                FULL_CONTEXT_TEMPLATE,
                "Alpha <Builder>",
                &[
                    TemplateSection::new(
                        "Commitments",
                        vec!["Ship <fast> & keep \"trust\"".to_owned()],
                    ),
                    TemplateSection::new("Warnings", vec![]),
                    TemplateSection::new(
                        "Relationships",
                        vec!["operator note: primary 'owner' & ally".to_owned()],
                    ),
                ],
            )
            .expect("full context should render");

        assert_eq!(
            rendered,
            "Alpha &lt;Builder&gt;\n## Commitments\n- Ship &lt;fast&gt; &amp; keep &quot;trust&quot;\n\n## Relationships\n- operator note: primary &#39;owner&#39; &amp; ally"
        );
    }

    #[test]
    fn explain_template_omits_section_block_when_everything_is_missing() {
        let rendered = TemplateService::default()
            .render_explain(
                EXPLAIN_TEMPLATE,
                "Explain",
                &[TemplateSection::new("Warnings", vec![])],
            )
            .expect("explain should render");

        assert_eq!(rendered, "Explain");
    }

    #[test]
    fn unknown_template_name_fails_explicitly() {
        let error = TemplateService::default()
            .render_prompt_prefix("unknown-template", ComposeMode::Normal, "Alpha", 512)
            .expect_err("unknown template should fail");

        assert!(matches!(
            error,
            SoulError::TemplateLoad {
                template: "built-in",
                ..
            }
        ));
    }
}
