#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceReport {
    pub contributors: Vec<String>,
    pub config_hash: String,
}

impl ProvenanceReport {
    pub fn bootstrap() -> Self {
        Self {
            contributors: vec![
                "app".to_string(),
                "sources".to_string(),
                "services".to_string(),
            ],
            config_hash: "bootstrap".to_string(),
        }
    }
}
