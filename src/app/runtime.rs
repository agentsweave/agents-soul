use crate::{
    app::{config::ApplicationConfig, deps::SoulDependencies},
    cli,
    domain::SoulError,
};

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

#[derive(Debug, Clone, Default)]
pub struct SoulRuntime {
    config: ApplicationConfig,
    deps: SoulDependencies,
}

impl SoulRuntime {
    pub fn new(config: ApplicationConfig, deps: SoulDependencies) -> Self {
        Self { config, deps }
    }

    pub fn config(&self) -> &ApplicationConfig {
        &self.config
    }

    pub fn deps(&self) -> &SoulDependencies {
        &self.deps
    }

    pub fn run(&self) -> Result<(), SoulError> {
        cli::run()
    }
}

pub fn core_layers() -> Vec<CrateLayer> {
    vec![
        CrateLayer::App,
        CrateLayer::Domain,
        CrateLayer::Sources,
        CrateLayer::Services,
        CrateLayer::Adaptation,
        CrateLayer::Storage,
    ]
}

pub fn transport_layers() -> Vec<CrateLayer> {
    vec![CrateLayer::Cli, CrateLayer::Api, CrateLayer::Mcp]
}

pub fn crate_layout() -> Vec<CrateLayer> {
    let mut layers = core_layers();
    layers.extend(transport_layers());
    layers
}

pub fn run() -> Result<(), SoulError> {
    SoulRuntime::default().run()
}
