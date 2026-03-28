use crate::{
    api::ApiSurface,
    app::{config::ApplicationConfig, deps::SoulDependencies},
    domain::{BehavioralContext, ComposeRequest, SoulError},
    mcp::McpSurface,
};

const ALL_LAYERS: [CrateLayer; 9] = [
    CrateLayer::App,
    CrateLayer::Domain,
    CrateLayer::Sources,
    CrateLayer::Services,
    CrateLayer::Adaptation,
    CrateLayer::Storage,
    CrateLayer::Cli,
    CrateLayer::Api,
    CrateLayer::Mcp,
];

const CORE_LAYERS: [CrateLayer; 6] = [
    CrateLayer::App,
    CrateLayer::Domain,
    CrateLayer::Sources,
    CrateLayer::Services,
    CrateLayer::Adaptation,
    CrateLayer::Storage,
];

const TRANSPORT_LAYERS: [CrateLayer; 3] = [CrateLayer::Cli, CrateLayer::Api, CrateLayer::Mcp];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateLayer {
    App,
    Domain,
    Sources,
    Services,
    Adaptation,
    Storage,
    Cli,
    Api,
    Mcp,
}

impl CrateLayer {
    pub const fn name(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Domain => "domain",
            Self::Sources => "sources",
            Self::Services => "services",
            Self::Adaptation => "adaptation",
            Self::Storage => "storage",
            Self::Cli => "cli",
            Self::Api => "api",
            Self::Mcp => "mcp",
        }
    }
}

pub const fn crate_layout() -> &'static [CrateLayer] {
    &ALL_LAYERS
}

pub const fn core_layers() -> &'static [CrateLayer] {
    &CORE_LAYERS
}

pub const fn transport_layers() -> &'static [CrateLayer] {
    &TRANSPORT_LAYERS
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulRuntime {
    pub config: ApplicationConfig,
    pub deps: SoulDependencies,
    pub api: ApiSurface,
    pub mcp: McpSurface,
}

impl SoulRuntime {
    pub fn new(config: ApplicationConfig) -> Self {
        let deps = SoulDependencies::default();
        let api = ApiSurface::from_services(&deps.services);
        let mcp = McpSurface::from_services(&deps.services);

        Self {
            config,
            deps,
            api,
            mcp,
        }
    }

    pub fn compose(&self, request: ComposeRequest) -> Result<BehavioralContext, SoulError> {
        self.deps.services.compose.compose(request)
    }
}

impl Default for SoulRuntime {
    fn default() -> Self {
        Self::new(ApplicationConfig::default())
    }
}
