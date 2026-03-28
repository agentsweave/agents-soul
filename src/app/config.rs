use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::domain::{SoulConfig, SoulError};

pub const SOUL_CONFIG_FILE: &str = "soul.toml";
pub const SOUL_STATE_DIR: &str = ".soul";
pub const ADAPTATION_DB_FILE: &str = "patterns.sqlite";
pub const CONTEXT_CACHE_FILE: &str = "context_cache.json";
pub const ADAPTATION_LOG_FILE: &str = "adaptation_log.jsonl";

/// Paths derived from a soul workspace root.
///
/// The workspace input is always the directory that owns `soul.toml`; callers do not
/// pass the file path itself, and the loader never walks parent directories to search
/// for config. All derived state remains inside `<workspace>/.soul/`.
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
}

pub fn load_soul_config(workspace_root: impl Into<PathBuf>) -> Result<SoulConfig, SoulError> {
    let paths = WorkspacePaths::new(workspace_root);
    validate_workspace_root(paths.root())?;

    let config_path = paths.config_path();
    let raw = fs::read_to_string(&config_path).map_err(|error| SoulError::ConfigRead {
        path: config_path.display().to_string(),
        message: error.to_string(),
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

#[cfg(test)]
mod tests {
    use super::{WorkspacePaths, load_soul_config};
    use crate::domain::{
        OfflineRegistryBehavior, RevokedBehavior, SoulError, SoulLimits, TemplateConfig,
    };
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn workspace_paths_follow_documented_layout() {
        let paths = WorkspacePaths::new("/tmp/example-soul");

        assert_eq!(
            paths.config_path().to_string_lossy(),
            "/tmp/example-soul/soul.toml"
        );
        assert_eq!(
            paths.adaptation_db_path().to_string_lossy(),
            "/tmp/example-soul/.soul/patterns.sqlite"
        );
        assert_eq!(
            paths.context_cache_path().to_string_lossy(),
            "/tmp/example-soul/.soul/context_cache.json"
        );
        assert_eq!(
            paths.adaptation_log_path().to_string_lossy(),
            "/tmp/example-soul/.soul/adaptation_log.jsonl"
        );
    }

    #[test]
    fn load_soul_config_materializes_defaults_deterministically() {
        let workspace = TestWorkspace::new();
        workspace.write_config(
            r#"
schema_version = 1
agent_id = "alpha"
profile_name = "Alpha Builder"

[sources]
identity_workspace = "~/.agents/alpha"
registry_url = "http://127.0.0.1:7700"
"#,
        );

        let config = load_soul_config(workspace.path()).expect("config should load");

        assert_eq!(config.sources.registry_agent_id, "alpha");
        assert_eq!(config.limits, SoulLimits::default());
        assert_eq!(config.templates, TemplateConfig::default());
        assert!(config.adaptation.enabled);
        assert_eq!(config.adaptation.learning_window_days, 30);
        assert_eq!(config.adaptation.min_interactions_for_adapt, 5);
        assert_eq!(
            config.limits.offline_registry_behavior,
            OfflineRegistryBehavior::Cautious
        );
        assert_eq!(config.limits.revoked_behavior, RevokedBehavior::FailClosed);
    }

    #[test]
    fn load_soul_config_rejects_workspace_file_paths() {
        let workspace = TestWorkspace::new();
        workspace.write_config(
            r#"
schema_version = 1
agent_id = "alpha"
profile_name = "Alpha Builder"

[sources]
identity_workspace = "~/.agents/alpha"
registry_url = "http://127.0.0.1:7700"
"#,
        );

        let error =
            load_soul_config(workspace.path().join("soul.toml")).expect_err("file paths fail");

        assert!(
            matches!(error, SoulError::InvalidConfig(message) if message.contains("directory containing `soul.toml`"))
        );
    }

    #[test]
    fn load_soul_config_reports_parse_failures_with_file_path() {
        let workspace = TestWorkspace::new();
        workspace.write_config(
            r#"
schema_version = 1
agent_id = "alpha"
profile_name = "Alpha Builder"

[sources]
identity_workspace = "~/.agents/alpha"
registry_url = 42
"#,
        );

        let error = load_soul_config(workspace.path()).expect_err("parse failure expected");

        assert!(
            matches!(error, SoulError::ConfigParse { path, message } if path.ends_with("soul.toml") && message.contains("registry_url"))
        );
    }

    #[test]
    fn load_soul_config_reports_validation_failures_with_actionable_fields() {
        let workspace = TestWorkspace::new();
        workspace.write_config(
            r#"
schema_version = 1
agent_id = "alpha"
profile_name = "Alpha Builder"

[sources]
identity_workspace = "~/.agents/alpha"
registry_url = "registry.internal"

[adaptation]
min_interactions_for_adapt = 0
"#,
        );

        let error = load_soul_config(workspace.path()).expect_err("validation failure expected");

        assert!(
            matches!(error, SoulError::InvalidConfig(message) if message.contains("sources.registry_url") || message.contains("adaptation.min_interactions_for_adapt"))
        );
    }

    struct TestWorkspace {
        root: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);

            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be monotonic enough for tests")
                .as_nanos();
            let root = env::temp_dir().join(format!("agents-soul-config-{nanos}-{unique}"));

            fs::create_dir_all(&root).expect("temp workspace should be created");
            Self { root }
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn write_config(&self, contents: &str) {
            fs::write(self.root.join("soul.toml"), contents).expect("config should be written");
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
