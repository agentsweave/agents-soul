use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::domain::{SoulConfig, SoulError};

pub const SOUL_CONFIG_FILE: &str = "soul.toml";
pub const SOUL_STATE_DIR: &str = ".soul";
pub const ADAPTATION_DB_FILE: &str = "patterns.sqlite";
pub const CONTEXT_CACHE_FILE: &str = "context_cache.json";
pub const ADAPTATION_LOG_FILE: &str = "adaptation_log.jsonl";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceContractPaths {
    config_path: PathBuf,
    adaptation_db_path: PathBuf,
    adaptation_log_path: PathBuf,
}

impl WorkspaceContractPaths {
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn adaptation_db_path(&self) -> &Path {
        &self.adaptation_db_path
    }

    pub fn adaptation_log_path(&self) -> &Path {
        &self.adaptation_log_path
    }

    pub fn required_files(&self) -> Vec<PathBuf> {
        vec![
            self.config_path.clone(),
            self.adaptation_db_path.clone(),
            self.adaptation_log_path.clone(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePaths {
    workspace_root: PathBuf,
}

impl WorkspacePaths {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn config_path(&self) -> PathBuf {
        self.workspace_root.join(SOUL_CONFIG_FILE)
    }

    pub fn state_dir(&self) -> PathBuf {
        self.workspace_root.join(SOUL_STATE_DIR)
    }

    pub fn adaptation_db_path(&self) -> PathBuf {
        self.state_dir().join(ADAPTATION_DB_FILE)
    }

    pub fn context_cache_path(&self) -> PathBuf {
        self.state_dir().join(CONTEXT_CACHE_FILE)
    }

    pub fn adaptation_log_path(&self) -> PathBuf {
        self.state_dir().join(ADAPTATION_LOG_FILE)
    }

    pub fn contract_paths(&self) -> WorkspaceContractPaths {
        WorkspaceContractPaths {
            config_path: self.config_path(),
            adaptation_db_path: self.adaptation_db_path(),
            adaptation_log_path: self.adaptation_log_path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationConfig {
    workspace_root: PathBuf,
}

impl ApplicationConfig {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn workspace_paths(&self) -> WorkspacePaths {
        WorkspacePaths::new(self.workspace_root.clone())
    }

    pub fn load_soul_config(&self) -> Result<SoulConfig, SoulError> {
        load_soul_config(self.workspace_root.clone())
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self::new(env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

pub fn load_soul_config(workspace_root: impl Into<PathBuf>) -> Result<SoulConfig, SoulError> {
    let paths = WorkspacePaths::new(workspace_root);
    validate_workspace_root(paths.root())?;

    let config_path = paths.config_path();
    let path = config_path.display().to_string();
    let raw = fs::read_to_string(&config_path).map_err(|error| {
        let message = match error.kind() {
            ErrorKind::NotFound => format!(
                "required soul config `soul.toml` is missing; create `{path}` in the workspace root"
            ),
            ErrorKind::PermissionDenied => {
                format!("permission denied while reading `{path}`; ensure the file is readable")
            }
            _ => error.to_string(),
        };

        SoulError::ConfigRead { path, message }
    })?;

    let parsed = toml::from_str::<SoulConfig>(&raw).map_err(|error| SoulError::ConfigParse {
        path: config_path.display().to_string(),
        message: error.to_string(),
    })?;

    parsed.finalize()
}

fn validate_workspace_root(workspace_root: &Path) -> Result<(), SoulError> {
    if workspace_root
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value == SOUL_CONFIG_FILE)
    {
        return Err(SoulError::InvalidConfig(format!(
            "workspace path must be the directory containing `{SOUL_CONFIG_FILE}`, got `{}`",
            workspace_root.display()
        )));
    }

    Ok(())
}
