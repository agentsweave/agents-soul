#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layer {
    name: &'static str,
}

impl Layer {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }
}

pub fn init_tracing() {}

pub fn crate_layout() -> Vec<Layer> {
    vec![
        Layer::new("app"),
        Layer::new("domain"),
        Layer::new("sources"),
        Layer::new("services"),
        Layer::new("adaptation"),
        Layer::new("storage"),
        Layer::new("cli"),
        Layer::new("api"),
        Layer::new("mcp"),
    ]
}

pub fn transport_layers() -> Vec<Layer> {
    vec![Layer::new("cli"), Layer::new("api"), Layer::new("mcp")]
}

pub fn run() -> Result<(), SoulError> {
    cli::run()
}
use crate::{cli, domain::SoulError};
