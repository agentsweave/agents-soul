use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{CommunicationStyle, DecisionHeuristic, PersonalityProfile, SoulError, SoulLimits};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateConfig {
    #[serde(default = "default_prompt_prefix_template")]
    pub prompt_prefix_template: String,
    #[serde(default = "default_full_context_template")]
    pub full_context_template: String,
    #[serde(default = "default_explain_template")]
    pub explain_template: String,
}

impl TemplateConfig {
    pub fn validate(&self) -> Result<(), SoulError> {
        if self.prompt_prefix_template.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "templates.prompt_prefix_template must not be empty".into(),
            ));
        }
        if self.full_context_template.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "templates.full_context_template must not be empty".into(),
            ));
        }
        if self.explain_template.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "templates.explain_template must not be empty".into(),
            ));
        }
        Ok(())
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            prompt_prefix_template: default_prompt_prefix_template(),
            full_context_template: default_full_context_template(),
            explain_template: default_explain_template(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoulConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub agent_id: String,
    pub profile_name: String,
    #[serde(default)]
    pub trait_baseline: PersonalityProfile,
    #[serde(default)]
    pub communication_style: CommunicationStyle,
    #[serde(default)]
    pub decision_heuristics: Vec<DecisionHeuristic>,
    #[serde(default)]
    pub limits: SoulLimits,
    #[serde(default)]
    pub templates: TemplateConfig,
    pub sources: SourceConfig,
    #[serde(default)]
    pub adaptation: AdaptationConfig,
}

impl SoulConfig {
    pub fn finalize(mut self) -> Result<Self, SoulError> {
        self.materialize_defaults();
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), SoulError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(SoulError::InvalidConfig(format!(
                "schema_version {} is unsupported; expected {}",
                self.schema_version, CURRENT_SCHEMA_VERSION
            )));
        }
        if self.agent_id.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "agent_id must not be empty".into(),
            ));
        }
        if self.profile_name.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "profile_name must not be empty".into(),
            ));
        }

        self.trait_baseline.validate()?;
        self.limits.validate()?;
        self.templates.validate()?;
        self.sources.validate()?;
        self.adaptation.validate()?;

        let mut seen = HashSet::new();
        for heuristic in &self.decision_heuristics {
            heuristic.validate()?;
            if !seen.insert(heuristic.heuristic_id.clone()) {
                return Err(SoulError::InvalidConfig(format!(
                    "duplicate heuristic id `{}`",
                    heuristic.heuristic_id
                )));
            }
        }

        Ok(())
    }

    fn materialize_defaults(&mut self) {
        if self.sources.registry_agent_id.trim().is_empty() {
            self.sources.registry_agent_id = self.agent_id.clone();
        }
    }
}

impl Default for SoulConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            agent_id: "agent".into(),
            profile_name: "Default Soul".into(),
            trait_baseline: PersonalityProfile::default(),
            communication_style: CommunicationStyle::default(),
            decision_heuristics: Vec::new(),
            limits: SoulLimits::default(),
            templates: TemplateConfig::default(),
            sources: SourceConfig::default(),
            adaptation: AdaptationConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceConfig {
    pub identity_workspace: String,
    pub registry_url: String,
    #[serde(default)]
    pub registry_agent_id: String,
}

impl SourceConfig {
    pub fn validate(&self) -> Result<(), SoulError> {
        if self.identity_workspace.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "sources.identity_workspace must not be empty".into(),
            ));
        }
        if self.registry_url.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "sources.registry_url must not be empty".into(),
            ));
        }
        if !(self.registry_url.starts_with("http://") || self.registry_url.starts_with("https://"))
        {
            return Err(SoulError::InvalidConfig(format!(
                "sources.registry_url must start with http:// or https://, got `{}`",
                self.registry_url
            )));
        }
        if self.registry_agent_id.trim().is_empty() {
            return Err(SoulError::InvalidConfig(
                "sources.registry_agent_id must not be empty".into(),
            ));
        }
        Ok(())
    }
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            identity_workspace: "~/.agents/default".into(),
            registry_url: "http://127.0.0.1:7700".into(),
            registry_agent_id: "agent".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptationConfig {
    #[serde(default = "default_adaptation_enabled")]
    pub enabled: bool,
    #[serde(default = "default_learning_window_days")]
    pub learning_window_days: u32,
    #[serde(default = "default_min_interactions_for_adapt")]
    pub min_interactions_for_adapt: u32,
    #[serde(default = "default_min_persist_interval_seconds")]
    pub min_persist_interval_seconds: u64,
}

impl AdaptationConfig {
    pub fn validate(&self) -> Result<(), SoulError> {
        if self.learning_window_days == 0 {
            return Err(SoulError::InvalidConfig(
                "adaptation.learning_window_days must be greater than zero".into(),
            ));
        }
        if self.min_interactions_for_adapt == 0 {
            return Err(SoulError::InvalidConfig(
                "adaptation.min_interactions_for_adapt must be greater than zero".into(),
            ));
        }
        Ok(())
    }
}

impl Default for AdaptationConfig {
    fn default() -> Self {
        Self {
            enabled: default_adaptation_enabled(),
            learning_window_days: default_learning_window_days(),
            min_interactions_for_adapt: default_min_interactions_for_adapt(),
            min_persist_interval_seconds: default_min_persist_interval_seconds(),
        }
    }
}

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

fn default_prompt_prefix_template() -> String {
    "prompt-prefix".into()
}

fn default_full_context_template() -> String {
    "full-context".into()
}

fn default_explain_template() -> String {
    "explain".into()
}

const fn default_adaptation_enabled() -> bool {
    true
}

const fn default_learning_window_days() -> u32 {
    30
}

const fn default_min_interactions_for_adapt() -> u32 {
    5
}

const fn default_min_persist_interval_seconds() -> u64 {
    300
}
