use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::SoulError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub version: i64,
    pub description: &'static str,
    pub statements: &'static [&'static str],
}

const CREATE_MIGRATIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL,
    description TEXT NOT NULL
)
"#;

const V1_STATEMENTS: &[&str] = &[
    r#"
CREATE TABLE IF NOT EXISTS interaction_events (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id         TEXT NOT NULL UNIQUE,
    agent_id         TEXT NOT NULL,
    session_id       TEXT,
    interaction_type TEXT NOT NULL,
    outcome          TEXT NOT NULL,
    signals_json     TEXT NOT NULL DEFAULT '[]',
    context_json     TEXT NOT NULL DEFAULT '{}',
    notes            TEXT,
    recorded_at      TEXT NOT NULL,
    created_at       TEXT NOT NULL
)
"#,
    r#"
CREATE INDEX IF NOT EXISTS idx_interaction_events_agent_recorded_at
ON interaction_events(agent_id, recorded_at DESC)
"#,
    r#"
CREATE INDEX IF NOT EXISTS idx_interaction_events_type_recorded_at
ON interaction_events(agent_id, interaction_type, recorded_at DESC)
"#,
    r#"
CREATE INDEX IF NOT EXISTS idx_interaction_events_outcome_recorded_at
ON interaction_events(agent_id, outcome, recorded_at DESC)
"#,
    r#"
CREATE TABLE IF NOT EXISTS adaptation_state (
    agent_id                      TEXT PRIMARY KEY,
    trait_overrides_json          TEXT NOT NULL DEFAULT '{}',
    communication_overrides_json  TEXT NOT NULL DEFAULT '{}',
    heuristic_overrides_json      TEXT NOT NULL DEFAULT '[]',
    notes_json                    TEXT NOT NULL DEFAULT '[]',
    evidence_window_size          INTEGER NOT NULL DEFAULT 20,
    interaction_count             INTEGER NOT NULL DEFAULT 0,
    last_interaction_at           TEXT,
    last_reset_at                 TEXT,
    updated_at                    TEXT NOT NULL
)
"#,
    r#"
CREATE INDEX IF NOT EXISTS idx_adaptation_state_updated_at
ON adaptation_state(updated_at DESC)
"#,
    r#"
CREATE TABLE IF NOT EXISTS adaptation_resets (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    reset_id    TEXT NOT NULL UNIQUE,
    agent_id    TEXT NOT NULL,
    reset_scope TEXT NOT NULL,
    target_key  TEXT,
    notes       TEXT,
    recorded_at TEXT NOT NULL,
    created_at  TEXT NOT NULL
)
"#,
    r#"
CREATE INDEX IF NOT EXISTS idx_adaptation_resets_agent_recorded_at
ON adaptation_resets(agent_id, recorded_at DESC)
"#,
    r#"
CREATE INDEX IF NOT EXISTS idx_adaptation_resets_scope
ON adaptation_resets(agent_id, reset_scope, target_key, recorded_at DESC)
"#,
];

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "create adaptive interaction evidence, effective state, and reset tables",
    statements: V1_STATEMENTS,
}];

pub fn apply_all(conn: &Connection) -> Result<(), SoulError> {
    conn.execute_batch(CREATE_MIGRATIONS_TABLE)
        .map_err(storage_error)?;

    for migration in MIGRATIONS {
        if migration_applied(conn, migration.version)? {
            continue;
        }

        apply_migration(conn, migration)?;
    }

    Ok(())
}

pub fn all() -> &'static [Migration] {
    MIGRATIONS
}

fn migration_applied(conn: &Connection, version: i64) -> Result<bool, SoulError> {
    conn.query_row(
        "SELECT 1 FROM schema_migrations WHERE version = ?1 LIMIT 1",
        params![version],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(storage_error)
}

fn apply_migration(conn: &Connection, migration: &Migration) -> Result<(), SoulError> {
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(storage_error)?;

    let result = (|| {
        for statement in migration.statements {
            conn.execute_batch(statement).map_err(storage_error)?;
        }

        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
            params![migration.version, now_sql(), migration.description],
        )
        .map_err(storage_error)?;

        Ok(())
    })();

    match result {
        Ok(()) => conn.execute_batch("COMMIT").map_err(storage_error),
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

pub(crate) fn now_sql() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn storage_error(error: rusqlite::Error) -> SoulError {
    SoulError::Storage(error.to_string())
}
