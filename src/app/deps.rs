use std::{fmt, sync::Arc};

use chrono::{DateTime, Utc};

use crate::{
    adaptation::{
        AdaptiveResetRequest, AdaptiveResetResult, EffectiveOverrideSet, InteractionRecordRequest,
        InteractionRecordResult, read_workspace_effective_overrides, record_workspace_interaction,
        reset_workspace_adaptation_state,
    },
    app::config::load_soul_config,
    app::errors::{SoulTransportError, map_soul_error},
    domain::{
        BehavioralContext, ComposeMode, ComposeRequest, IdentifySignals, ReputationSummary,
        SessionIdentitySnapshot, SoulConfig, SoulConfigPatch, SoulError, VerificationResult,
    },
    services::{
        SoulServices,
        provenance::{ProvenanceHasher, StableProvenanceHasher},
        templates::{PromptTemplateRenderer, TemplateService},
    },
    sources::{ReaderSelection, identity::IdentityReader, registry::RegistryReader},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceDependencies {
    pub identity: IdentityReader,
    pub registry: RegistryReader,
}

pub trait SoulConfigLoader: Send + Sync {
    fn load(&self, workspace_root: &str) -> Result<SoulConfig, SoulError>;
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceSoulConfigLoader;

impl SoulConfigLoader for WorkspaceSoulConfigLoader {
    fn load(&self, workspace_root: &str) -> Result<SoulConfig, SoulError> {
        load_soul_config(workspace_root)
    }
}

pub trait AdaptationStateLoader: Send + Sync {
    fn load_effective_overrides(
        &self,
        workspace_root: &str,
        config: &SoulConfig,
        agent_id: &str,
    ) -> Result<EffectiveOverrideSet, SoulError>;
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceAdaptationStateLoader;

impl AdaptationStateLoader for WorkspaceAdaptationStateLoader {
    fn load_effective_overrides(
        &self,
        workspace_root: &str,
        config: &SoulConfig,
        agent_id: &str,
    ) -> Result<EffectiveOverrideSet, SoulError> {
        read_workspace_effective_overrides(workspace_root, config, agent_id)
    }
}

pub trait ComposeClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Clone, Default)]
pub struct SystemClock;

impl ComposeClock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone)]
pub struct AppDeps {
    pub services: SoulServices,
    pub sources: SourceDependencies,
    config_loader: Arc<dyn SoulConfigLoader>,
    adaptation_loader: Arc<dyn AdaptationStateLoader>,
    template_renderer: Arc<dyn PromptTemplateRenderer>,
    clock: Arc<dyn ComposeClock>,
    provenance_hasher: Arc<dyn ProvenanceHasher>,
}

impl AppDeps {
    pub fn new(services: SoulServices, sources: SourceDependencies) -> Self {
        Self {
            services,
            sources,
            config_loader: Arc::new(WorkspaceSoulConfigLoader),
            adaptation_loader: Arc::new(WorkspaceAdaptationStateLoader),
            template_renderer: Arc::new(TemplateService::default()),
            clock: Arc::new(SystemClock),
            provenance_hasher: Arc::new(StableProvenanceHasher),
        }
    }

    pub fn with_sources(mut self, sources: SourceDependencies) -> Self {
        self.sources = sources;
        self
    }

    pub fn with_config_loader<L>(mut self, loader: L) -> Self
    where
        L: SoulConfigLoader + 'static,
    {
        self.config_loader = Arc::new(loader);
        self
    }

    pub fn with_adaptation_loader<L>(mut self, loader: L) -> Self
    where
        L: AdaptationStateLoader + 'static,
    {
        self.adaptation_loader = Arc::new(loader);
        self
    }

    pub fn with_template_renderer<R>(mut self, renderer: R) -> Self
    where
        R: PromptTemplateRenderer + 'static,
    {
        self.template_renderer = Arc::new(renderer);
        self
    }

    pub fn with_clock<C>(mut self, clock: C) -> Self
    where
        C: ComposeClock + 'static,
    {
        self.clock = Arc::new(clock);
        self
    }

    pub fn with_provenance_hasher<H>(mut self, hasher: H) -> Self
    where
        H: ProvenanceHasher + 'static,
    {
        self.provenance_hasher = Arc::new(hasher);
        self
    }

    pub fn compose_context(&self, request: ComposeRequest) -> Result<BehavioralContext, SoulError> {
        self.services.compose.compose(self, request)
    }

    pub fn inspect_report(
        &self,
        request: ComposeRequest,
    ) -> Result<crate::services::explain::InspectReport, SoulError> {
        let artifacts = self.services.compose.compose_artifacts(self, request)?;
        Ok(crate::services::ExplainService.build_inspect_report(
            &artifacts.normalized,
            &artifacts.effective_overrides,
            &artifacts.context,
        ))
    }

    pub fn update_soul_config(
        &self,
        workspace_root: impl Into<std::path::PathBuf>,
        patch: &SoulConfigPatch,
    ) -> Result<SoulConfig, SoulError> {
        self.services
            .workspace_config
            .patch_workspace(workspace_root, patch)
    }

    pub fn record_interaction(
        &self,
        workspace_root: impl Into<std::path::PathBuf>,
        request: &InteractionRecordRequest,
    ) -> Result<InteractionRecordResult, SoulError> {
        let workspace_root = workspace_root.into();
        let config = self.load_soul_config(&workspace_root.display().to_string())?;
        record_workspace_interaction(&workspace_root, &config, request)
    }

    pub fn reset_adaptation_state(
        &self,
        workspace_root: impl Into<std::path::PathBuf>,
        request: &AdaptiveResetRequest,
    ) -> Result<AdaptiveResetResult, SoulError> {
        reset_workspace_adaptation_state(workspace_root.into(), request)
    }

    pub fn map_error(&self, error: &SoulError) -> SoulTransportError {
        map_soul_error(error)
    }

    pub fn load_soul_config(&self, workspace_root: &str) -> Result<SoulConfig, SoulError> {
        self.config_loader.load(workspace_root)
    }

    pub fn load_effective_overrides(
        &self,
        workspace_root: &str,
        config: &SoulConfig,
        agent_id: &str,
    ) -> Result<EffectiveOverrideSet, SoulError> {
        self.adaptation_loader
            .load_effective_overrides(workspace_root, config, agent_id)
    }

    pub fn load_identify_signals(
        &self,
        request: &ComposeRequest,
        config: &SoulConfig,
    ) -> Result<ReaderSelection<IdentifySignals>, SoulError> {
        self.sources.identity.load(request, config)
    }

    pub fn load_identity_snapshot(
        &self,
        request: &ComposeRequest,
        config: &SoulConfig,
    ) -> Result<ReaderSelection<SessionIdentitySnapshot>, SoulError> {
        let selection = self.load_identify_signals(request, config)?;
        Ok(ReaderSelection {
            value: selection.value.and_then(|signals| signals.snapshot),
            provenance: selection.provenance,
            warnings: selection.warnings,
        })
    }

    pub fn load_registry_verification(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<VerificationResult>, SoulError> {
        self.sources.registry.load_verification(request)
    }

    pub fn load_registry_reputation(
        &self,
        request: &ComposeRequest,
    ) -> Result<ReaderSelection<ReputationSummary>, SoulError> {
        self.sources.registry.load_reputation(request)
    }

    pub fn render_prompt_prefix(
        &self,
        template_name: &str,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> Result<String, SoulError> {
        self.template_renderer.render_prompt_prefix(
            template_name,
            compose_mode,
            profile_name,
            max_chars,
        )
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    pub fn provenance_hasher(&self) -> &dyn ProvenanceHasher {
        self.provenance_hasher.as_ref()
    }
}

impl Default for AppDeps {
    fn default() -> Self {
        Self::new(SoulServices::default(), SourceDependencies::default())
    }
}

impl fmt::Debug for AppDeps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppDeps")
            .field("services", &self.services)
            .field("sources", &self.sources)
            .field("config_loader", &"dyn SoulConfigLoader")
            .field("adaptation_loader", &"dyn AdaptationStateLoader")
            .field("template_renderer", &"dyn PromptTemplateRenderer")
            .field("clock", &"dyn ComposeClock")
            .field("provenance_hasher", &"dyn ProvenanceHasher")
            .finish()
    }
}

pub type SoulDependencies = AppDeps;
