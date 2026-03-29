use std::env;

use agents_soul::app::deps::ComposeClock;
use agents_soul::domain::{
    AdaptationState, CURRENT_SCHEMA_VERSION, ComposeRequest, InputProvenance, NormalizedInputs,
    SessionIdentitySnapshot, SoulConfig,
};
use agents_soul::{
    BehavioralContext, ComposeMode, CrateLayer, SoulDependencies, SoulError, SoulErrorCategory,
    SoulRuntime,
    app::config::ApplicationConfig,
    core_layers, crate_layout,
    services::{provenance::ProvenanceHasher, templates::PromptTemplateRenderer},
    transport_layers,
};
use chrono::{DateTime, TimeZone, Utc};

#[test]
fn bootstrap_surface_exposes_core_contract_types() {
    let context = BehavioralContext::default();
    let runtime = SoulRuntime::default();
    let deps = SoulDependencies::default();
    let error = agents_soul::map_soul_error(&SoulError::RegistryUnavailable);

    assert!(context.system_prompt_prefix.is_empty());
    assert_eq!(
        context.status_summary.compose_mode,
        ComposeMode::BaselineOnly
    );
    assert_eq!(
        runtime.config().workspace_paths().state_dir(),
        env::current_dir()
            .expect("cwd should resolve")
            .join(".soul")
    );
    assert_eq!(
        core_layers(),
        vec![
            CrateLayer::App,
            CrateLayer::Domain,
            CrateLayer::Sources,
            CrateLayer::Services,
            CrateLayer::Adaptation,
            CrateLayer::Storage,
        ]
    );
    assert_eq!(
        transport_layers(),
        vec![CrateLayer::Cli, CrateLayer::Api, CrateLayer::Mcp]
    );
    assert_eq!(crate_layout().len(), 9);
    assert_eq!(deps.sources.identity, Default::default());
    assert_eq!(deps.sources.registry, Default::default());
    assert_eq!(error.category, SoulErrorCategory::UpstreamUnavailable);
    assert_eq!(error.compose_mode_hint, Some(ComposeMode::Degraded));
}

#[derive(Debug, Clone)]
struct FixedClock;

impl ComposeClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 29, 8, 0, 0)
            .single()
            .expect("fixed timestamp should be valid")
    }
}

#[derive(Debug, Clone)]
struct FixedRenderer;

impl PromptTemplateRenderer for FixedRenderer {
    fn render_prompt_prefix(
        &self,
        compose_mode: ComposeMode,
        profile_name: &str,
        max_chars: usize,
    ) -> Result<String, SoulError> {
        Ok(format!(
            "runtime:{compose_mode:?}:{profile_name}:{max_chars}"
        ))
    }
}

#[derive(Debug, Clone)]
struct FixedHasher;

impl ProvenanceHasher for FixedHasher {
    fn identity_fingerprint(
        &self,
        _snapshot: &SessionIdentitySnapshot,
    ) -> Result<String, SoulError> {
        Ok("id_runtime".to_owned())
    }

    fn config_hash(&self, _config: &SoulConfig) -> Result<String, SoulError> {
        Ok("cfg_runtime".to_owned())
    }

    fn adaptation_hash(&self, _state: &AdaptationState) -> Result<String, SoulError> {
        Ok("adp_runtime".to_owned())
    }

    fn input_hash(&self, _normalized: &NormalizedInputs) -> Result<String, SoulError> {
        Ok("inp_runtime".to_owned())
    }
}

#[test]
fn runtime_dispatch_preserves_injected_config_and_deps() -> Result<(), SoulError> {
    let config = ApplicationConfig::new("/tmp/injected-soul");
    let deps = SoulDependencies::default()
        .with_clock(FixedClock)
        .with_template_renderer(FixedRenderer)
        .with_provenance_hasher(FixedHasher);
    let runtime = SoulRuntime::new(config.clone(), deps);

    runtime.dispatch_with(|seen_config, seen_deps| {
        let normalized = NormalizedInputs {
            schema_version: CURRENT_SCHEMA_VERSION,
            request: ComposeRequest::new("alpha", "session-1"),
            agent_id: "alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            compose_mode_hint: Some(ComposeMode::Normal),
            identity_snapshot: None,
            identity_provenance: InputProvenance::unavailable("not loaded"),
            verification_result: None,
            verification_provenance: InputProvenance::unavailable("not loaded"),
            reputation_summary: None,
            reputation_provenance: InputProvenance::unavailable("not loaded"),
            soul_config: SoulConfig::default(),
            adaptation_state: AdaptationState::default(),
            reader_warnings: Vec::new(),
            generated_at: seen_deps.now(),
        };

        assert_eq!(seen_config.workspace_root(), config.workspace_root());
        assert_eq!(
            seen_deps.now(),
            Utc.with_ymd_and_hms(2026, 3, 29, 8, 0, 0)
                .single()
                .expect("fixed timestamp should be valid")
        );
        assert_eq!(
            seen_deps.render_prompt_prefix(ComposeMode::Restricted, "Alpha", 32)?,
            "runtime:Restricted:Alpha:32"
        );
        assert_eq!(
            seen_deps
                .provenance_hasher()
                .config_hash(&SoulConfig::default())?,
            "cfg_runtime"
        );
        assert_eq!(
            seen_deps
                .provenance_hasher()
                .adaptation_hash(&AdaptationState::default())?,
            "adp_runtime"
        );
        assert_eq!(
            seen_deps.provenance_hasher().input_hash(&normalized)?,
            "inp_runtime"
        );
        Ok(())
    })
}
