use std::env;

use agents_soul::{
    BehavioralContext, ComposeMode, CrateLayer, SoulDependencies, SoulError, SoulErrorCategory,
    SoulRuntime, core_layers, crate_layout, transport_layers,
};

#[test]
fn bootstrap_surface_exposes_core_contract_types() {
    let context = BehavioralContext::default();
    let runtime = SoulRuntime::default();
    let deps = SoulDependencies::default();
    let error = SoulError::RegistryUnavailable.transport_error();

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
