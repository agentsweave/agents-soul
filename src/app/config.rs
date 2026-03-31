use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::domain::{SoulConfig, SoulError};

pub const SOUL_CONFIG_FILE: &str = "soul.toml";
pub const SOUL_CONFIG_DROPIN_DIR: &str = "soul.d";
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

    pub fn config_dropin_dir(&self) -> PathBuf {
        self.workspace_root.join(SOUL_CONFIG_DROPIN_DIR)
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
    let raw = read_required_config_file(&config_path)?;
    let mut merged = parse_config_value(&config_path, &raw)?;

    let dropin_dir = paths.config_dropin_dir();
    let entries = match fs::read_dir(&dropin_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return finalize_config_value(&config_path, merged);
        }
        Err(error) => {
            return Err(SoulError::ConfigRead {
                path: dropin_dir.display().to_string(),
                message: error.to_string(),
            });
        }
    };

    let mut files = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".toml") && !name.starts_with('.'))
        })
        .collect::<Vec<_>>();
    files.sort();

    for file in files {
        let overlay_raw = fs::read_to_string(&file).map_err(|error| SoulError::ConfigRead {
            path: file.display().to_string(),
            message: error.to_string(),
        })?;
        let overlay = parse_config_value(&file, &overlay_raw)?;
        merge_toml_value(&mut merged, overlay);
    }

    finalize_config_value(&config_path, merged)
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

fn read_required_config_file(path: &Path) -> Result<String, SoulError> {
    let display = path.display().to_string();
    fs::read_to_string(path).map_err(|error| {
        let message = match error.kind() {
            ErrorKind::NotFound => format!(
                "required soul config `soul.toml` is missing; create `{display}` in the workspace root"
            ),
            ErrorKind::PermissionDenied => {
                format!("permission denied while reading `{display}`; ensure the file is readable")
            }
            _ => error.to_string(),
        };

        SoulError::ConfigRead {
            path: display,
            message,
        }
    })
}

fn parse_config_value(path: &Path, raw: &str) -> Result<toml::Value, SoulError> {
    toml::from_str::<toml::Value>(raw).map_err(|error| SoulError::ConfigParse {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}

fn finalize_config_value(path: &Path, value: toml::Value) -> Result<SoulConfig, SoulError> {
    let parsed: SoulConfig = value.try_into().map_err(|error| SoulError::ConfigParse {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    parsed.finalize()
}

fn merge_toml_value(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, overlay_value) in overlay_table {
                match base_table.get_mut(&key) {
                    Some(base_value) => merge_toml_value(base_value, overlay_value),
                    None => {
                        base_table.insert(key, overlay_value);
                    }
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspacePaths, load_soul_config};
    use std::{fs, path::Path};
    use tempfile::tempdir;

    #[test]
    fn load_soul_config_merges_sorted_dropins() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let paths = WorkspacePaths::new(temp.path());
        fs::create_dir_all(paths.config_dropin_dir())?;
        fs::write(
            paths.config_path(),
            r#"
schema_version = 1
agent_id = "alpha"
profile_name = "Alpha"

[sources]
identity_workspace = "/tmp/identity"
registry_url = "http://127.0.0.1:7700"

[adaptation]
min_interactions_for_adapt = 5
"#,
        )?;
        fs::write(
            paths.config_dropin_dir().join("10-profile.toml"),
            r#"
[sources]
registry_agent_id = "alpha-registry"

[adaptation]
min_persist_interval_seconds = 120
"#,
        )?;
        fs::write(
            paths.config_dropin_dir().join("20-style.toml"),
            r#"
[communication_style]
default_register = "advisory"
"#,
        )?;

        let config = load_soul_config(temp.path())?;
        assert_eq!(config.sources.registry_agent_id, "alpha-registry");
        assert_eq!(config.adaptation.min_persist_interval_seconds, 120);
        assert_eq!(
            format!("{:?}", config.communication_style.default_register).to_lowercase(),
            "advisory"
        );
        Ok(())
    }

    #[test]
    fn workspace_paths_expose_config_dropin_dir() {
        let paths = WorkspacePaths::new(Path::new("/tmp/example-soul"));
        assert_eq!(
            paths.config_dropin_dir().to_string_lossy(),
            "/tmp/example-soul/soul.d"
        );
    }
}
