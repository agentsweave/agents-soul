use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::{
    app::config::WorkspacePaths,
    domain::SoulError,
    storage::sqlite::{self, AdaptationStateRecord},
};

static FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Deterministic, reusable storage fixture helpers for integration tests.
///
/// This lives in the main crate (not `cfg(test)`) so `tests/` integration tests can reuse it.
#[derive(Debug)]
pub struct StorageFixture {
    root: PathBuf,
    keep: bool,
}

impl StorageFixture {
    pub fn new(label: &str) -> Result<Self, SoulError> {
        let counter = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let root = std::env::temp_dir().join(format!("agents-soul-{label}-{pid}-{counter}"));
        fs::create_dir_all(&root).map_err(|error| SoulError::Storage(error.to_string()))?;
        Ok(Self { root, keep: false })
    }

    /// Prevent auto-cleanup on drop; useful when debugging failed tests locally.
    pub fn keep_on_drop(mut self) -> Self {
        self.keep = true;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn paths(&self) -> WorkspacePaths {
        WorkspacePaths::new(self.root.clone())
    }

    pub fn adaptation_db_path(&self) -> PathBuf {
        self.paths().adaptation_db_path()
    }

    pub fn open_adaptation_db(&self) -> Result<Connection, SoulError> {
        sqlite::open_database(self.adaptation_db_path())
    }

    pub fn write_relative(
        &self,
        relative: impl AsRef<Path>,
        content: &str,
    ) -> Result<PathBuf, SoulError> {
        let path = self.root.join(relative.as_ref());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| SoulError::Storage(error.to_string()))?;
        }
        fs::write(&path, content).map_err(|error| SoulError::Storage(error.to_string()))?;
        Ok(path)
    }
}

impl Drop for StorageFixture {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

pub fn timestamp_utc(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> Result<DateTime<Utc>, SoulError> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
        .ok_or_else(|| SoulError::Internal("invalid timestamp fixture parameters".to_owned()))
}

pub fn minimal_adaptation_state_record(
    agent_id: impl Into<String>,
    updated_at: DateTime<Utc>,
) -> AdaptationStateRecord {
    AdaptationStateRecord {
        agent_id: agent_id.into(),
        trait_overrides_json: "{}".to_owned(),
        communication_overrides_json: "{}".to_owned(),
        heuristic_overrides_json: "[]".to_owned(),
        notes_json: "[]".to_owned(),
        evidence_window_size: 10,
        interaction_count: 0,
        last_interaction_at: None,
        last_reset_at: None,
        updated_at,
    }
}
