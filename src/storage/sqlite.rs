use std::{fs, path::Path};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{CommunicationOverride, HeuristicOverride, PersonalityOverride, SoulError};

use super::migrations;

#[derive(Debug, Clone, PartialEq)]
pub struct InteractionEventRecord {
    pub event_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub interaction_type: String,
    pub outcome: String,
    pub signals_json: String,
    pub context_json: String,
    pub notes: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptationStateRecord {
    pub agent_id: String,
    pub trait_overrides_json: String,
    pub communication_overrides_json: String,
    pub heuristic_overrides_json: String,
    pub notes_json: String,
    pub evidence_window_size: u32,
    pub interaction_count: u32,
    pub last_interaction_at: Option<DateTime<Utc>>,
    pub last_reset_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetScope {
    All,
    Trait,
    Communication,
    Heuristic,
}

impl ResetScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Trait => "trait",
            Self::Communication => "communication",
            Self::Heuristic => "heuristic",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptationResetRecord {
    pub reset_id: String,
    pub agent_id: String,
    pub scope: ResetScope,
    pub target_key: Option<String>,
    pub notes: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, SoulError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| SoulError::Storage(error.to_string()))?;
    }

    let conn = Connection::open(path).map_err(storage_error)?;
    initialize_database(&conn)?;
    Ok(conn)
}

pub fn initialize_database(conn: &Connection) -> Result<(), SoulError> {
    migrations::apply_all(conn)
}

pub fn record_interaction_event(
    conn: &Connection,
    event: &InteractionEventRecord,
) -> Result<bool, SoulError> {
    initialize_database(conn)?;

    let inserted = conn
        .execute(
            r#"
INSERT OR IGNORE INTO interaction_events (
    event_id,
    agent_id,
    session_id,
    interaction_type,
    outcome,
    signals_json,
    context_json,
    notes,
    recorded_at,
    created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
"#,
            params![
                &event.event_id,
                &event.agent_id,
                event.session_id.as_deref(),
                &event.interaction_type,
                &event.outcome,
                &event.signals_json,
                &event.context_json,
                event.notes.as_deref(),
                event.recorded_at.to_rfc3339(),
                migrations::now_sql(),
            ],
        )
        .map_err(storage_error)?;

    Ok(inserted > 0)
}

pub fn load_adaptation_state(
    conn: &Connection,
    agent_id: &str,
) -> Result<Option<AdaptationStateRecord>, SoulError> {
    initialize_database(conn)?;

    conn.query_row(
        r#"
SELECT
    agent_id,
    trait_overrides_json,
    communication_overrides_json,
    heuristic_overrides_json,
    notes_json,
    evidence_window_size,
    interaction_count,
    last_interaction_at,
    last_reset_at,
    updated_at
FROM adaptation_state
WHERE agent_id = ?1
"#,
        params![agent_id],
        |row| {
            Ok(AdaptationStateRecord {
                agent_id: row.get(0)?,
                trait_overrides_json: row.get(1)?,
                communication_overrides_json: row.get(2)?,
                heuristic_overrides_json: row.get(3)?,
                notes_json: row.get(4)?,
                evidence_window_size: row.get(5)?,
                interaction_count: row.get(6)?,
                last_interaction_at: parse_optional_timestamp(row.get::<_, Option<String>>(7)?)?,
                last_reset_at: parse_optional_timestamp(row.get::<_, Option<String>>(8)?)?,
                updated_at: parse_timestamp(row.get::<_, String>(9)?)?,
            })
        },
    )
    .optional()
    .map_err(storage_error)
}

pub fn upsert_adaptation_state(
    conn: &Connection,
    state: &AdaptationStateRecord,
) -> Result<(), SoulError> {
    initialize_database(conn)?;

    conn.execute(
        r#"
INSERT INTO adaptation_state (
    agent_id,
    trait_overrides_json,
    communication_overrides_json,
    heuristic_overrides_json,
    notes_json,
    evidence_window_size,
    interaction_count,
    last_interaction_at,
    last_reset_at,
    updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
ON CONFLICT(agent_id) DO UPDATE SET
    trait_overrides_json = excluded.trait_overrides_json,
    communication_overrides_json = excluded.communication_overrides_json,
    heuristic_overrides_json = excluded.heuristic_overrides_json,
    notes_json = excluded.notes_json,
    evidence_window_size = excluded.evidence_window_size,
    interaction_count = excluded.interaction_count,
    last_interaction_at = excluded.last_interaction_at,
    last_reset_at = excluded.last_reset_at,
    updated_at = excluded.updated_at
"#,
        params![
            &state.agent_id,
            &state.trait_overrides_json,
            &state.communication_overrides_json,
            &state.heuristic_overrides_json,
            &state.notes_json,
            state.evidence_window_size,
            state.interaction_count,
            state.last_interaction_at.map(|value| value.to_rfc3339()),
            state.last_reset_at.map(|value| value.to_rfc3339()),
            state.updated_at.to_rfc3339(),
        ],
    )
    .map_err(storage_error)?;

    Ok(())
}

pub fn record_reset(conn: &Connection, reset: &AdaptationResetRecord) -> Result<bool, SoulError> {
    initialize_database(conn)?;

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(storage_error)?;

    let result = (|| {
        let inserted = conn
            .execute(
                r#"
INSERT OR IGNORE INTO adaptation_resets (
    reset_id,
    agent_id,
    reset_scope,
    target_key,
    notes,
    recorded_at,
    created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
"#,
                params![
                    &reset.reset_id,
                    &reset.agent_id,
                    reset.scope.as_str(),
                    reset.target_key.as_deref(),
                    reset.notes.as_deref(),
                    reset.recorded_at.to_rfc3339(),
                    migrations::now_sql(),
                ],
            )
            .map_err(storage_error)?;

        if inserted == 0 {
            return Ok(false);
        }

        match reset.scope {
            ResetScope::All => {
                conn.execute(
                    "DELETE FROM adaptation_state WHERE agent_id = ?1",
                    params![&reset.agent_id],
                )
                .map_err(storage_error)?;
            }
            ResetScope::Trait | ResetScope::Communication | ResetScope::Heuristic => {
                if let Some(mut state) = load_adaptation_state(conn, &reset.agent_id)? {
                    apply_targeted_reset(&mut state, reset)?;
                    upsert_adaptation_state(conn, &state)?;
                }
            }
        }

        Ok(true)
    })();

    match result {
        Ok(inserted) => {
            conn.execute_batch("COMMIT").map_err(storage_error)?;
            Ok(inserted)
        }
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn apply_targeted_reset(
    state: &mut AdaptationStateRecord,
    reset: &AdaptationResetRecord,
) -> Result<(), SoulError> {
    match reset.scope {
        ResetScope::All => {}
        ResetScope::Trait => {
            let mut overrides =
                deserialize_json::<PersonalityOverride>(&state.trait_overrides_json)?;
            if let Some(target_key) = reset.target_key.as_deref() {
                clear_trait_override(&mut overrides, target_key)?;
            } else {
                overrides = PersonalityOverride::default();
            }
            state.trait_overrides_json = serialize_json(&overrides)?;
        }
        ResetScope::Communication => {
            let mut overrides =
                deserialize_json::<CommunicationOverride>(&state.communication_overrides_json)?;
            if let Some(target_key) = reset.target_key.as_deref() {
                clear_communication_override(&mut overrides, target_key)?;
            } else {
                overrides = CommunicationOverride::default();
            }
            state.communication_overrides_json = serialize_json(&overrides)?;
        }
        ResetScope::Heuristic => {
            let mut overrides =
                deserialize_json::<Vec<HeuristicOverride>>(&state.heuristic_overrides_json)?;
            if let Some(target_key) = reset.target_key.as_deref() {
                overrides.retain(|override_rule| override_rule.heuristic_id != target_key);
            } else {
                overrides.clear();
            }
            state.heuristic_overrides_json = serialize_json(&overrides)?;
        }
    }

    // Notes are derived from adaptive effects; clear them so stale explanations do not survive a reset.
    state.notes_json = "[]".to_owned();
    state.last_reset_at = Some(reset.recorded_at);
    state.updated_at = reset.recorded_at;
    Ok(())
}

fn clear_trait_override(
    overrides: &mut PersonalityOverride,
    target_key: &str,
) -> Result<(), SoulError> {
    match target_key {
        "openness" => overrides.openness = 0.0,
        "conscientiousness" => overrides.conscientiousness = 0.0,
        "initiative" => overrides.initiative = 0.0,
        "directness" => overrides.directness = 0.0,
        "warmth" => overrides.warmth = 0.0,
        "risk_tolerance" => overrides.risk_tolerance = 0.0,
        "verbosity" => overrides.verbosity = 0.0,
        "formality" => overrides.formality = 0.0,
        _ => {
            return Err(SoulError::InvalidConfig(format!(
                "unknown trait reset target `{target_key}`"
            )));
        }
    }

    Ok(())
}

fn clear_communication_override(
    overrides: &mut CommunicationOverride,
    target_key: &str,
) -> Result<(), SoulError> {
    match target_key {
        "default_register" => overrides.default_register = None,
        "paragraph_budget" => overrides.paragraph_budget = None,
        "question_style" => overrides.question_style = None,
        "uncertainty_style" => overrides.uncertainty_style = None,
        "feedback_style" => overrides.feedback_style = None,
        "conflict_style" => overrides.conflict_style = None,
        _ => {
            return Err(SoulError::InvalidConfig(format!(
                "unknown communication reset target `{target_key}`"
            )));
        }
    }

    Ok(())
}

fn deserialize_json<T>(raw: &str) -> Result<T, SoulError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(raw).map_err(|error| SoulError::Storage(error.to_string()))
}

fn serialize_json<T>(value: &T) -> Result<String, SoulError>
where
    T: serde::Serialize,
{
    serde_json::to_string(value).map_err(|error| SoulError::Storage(error.to_string()))
}

fn parse_optional_timestamp(
    value: Option<String>,
) -> Result<Option<DateTime<Utc>>, rusqlite::Error> {
    value.map(parse_timestamp).transpose()
}

fn parse_timestamp(value: String) -> Result<DateTime<Utc>, rusqlite::Error> {
    DateTime::parse_from_rfc3339(&value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn storage_error(error: rusqlite::Error) -> SoulError {
    SoulError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AdaptationResetRecord, AdaptationStateRecord, InteractionEventRecord, ResetScope,
        initialize_database, record_interaction_event, record_reset, upsert_adaptation_state,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use rusqlite::Connection;
    use std::{error::Error, io};

    #[test]
    fn migrations_create_expected_tables_and_indexes() -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;

        initialize_database(&conn)?;

        for name in [
            "schema_migrations",
            "interaction_events",
            "adaptation_state",
            "adaptation_resets",
            "idx_interaction_events_agent_recorded_at",
            "idx_interaction_events_type_recorded_at",
            "idx_interaction_events_outcome_recorded_at",
            "idx_adaptation_state_updated_at",
            "idx_adaptation_resets_agent_recorded_at",
            "idx_adaptation_resets_scope",
        ] {
            ensure(
                sqlite_object_exists(&conn, name)?,
                format!("missing sqlite object: {name}"),
            )?;
        }

        Ok(())
    }

    #[test]
    fn migrations_are_idempotent() -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;

        initialize_database(&conn)?;
        initialize_database(&conn)?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;

        ensure(
            count == 1,
            format!("expected one migration row, got {count}"),
        )?;
        Ok(())
    }

    #[test]
    fn duplicate_interaction_events_are_idempotent() -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;
        initialize_database(&conn)?;

        let event = InteractionEventRecord {
            event_id: "evt-1".to_owned(),
            agent_id: "agent.alpha".to_owned(),
            session_id: Some("session-1".to_owned()),
            interaction_type: "communication".to_owned(),
            outcome: "positive".to_owned(),
            signals_json: r#"[{"trait":"verbosity","delta":-0.03}]"#.to_owned(),
            context_json: r#"{"channel":"cli"}"#.to_owned(),
            notes: Some("User preferred concise output.".to_owned()),
            recorded_at: test_timestamp(2026, 3, 29, 1, 0, 0)?,
        };

        ensure(
            record_interaction_event(&conn, &event)?,
            "expected first interaction insert to succeed",
        )?;
        ensure(
            !record_interaction_event(&conn, &event)?,
            "expected duplicate interaction insert to be ignored",
        )?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM interaction_events", [], |row| {
            row.get(0)
        })?;

        ensure(
            count == 1,
            format!("expected one interaction row, got {count}"),
        )?;
        Ok(())
    }

    #[test]
    fn reset_markers_clear_materialized_state_but_preserve_interaction_evidence()
    -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;
        initialize_database(&conn)?;

        let state = AdaptationStateRecord {
            agent_id: "agent.alpha".to_owned(),
            trait_overrides_json: r#"{"verbosity":-0.05}"#.to_owned(),
            communication_overrides_json: "{}".to_owned(),
            heuristic_overrides_json: "[]".to_owned(),
            notes_json: r#"["Verbosity reduced after repeated positive feedback."]"#.to_owned(),
            evidence_window_size: 20,
            interaction_count: 8,
            last_interaction_at: Some(test_timestamp(2026, 3, 29, 0, 55, 0)?),
            last_reset_at: None,
            updated_at: test_timestamp(2026, 3, 29, 1, 0, 0)?,
        };
        upsert_adaptation_state(&conn, &state)?;

        let event = InteractionEventRecord {
            event_id: "evt-2".to_owned(),
            agent_id: "agent.alpha".to_owned(),
            session_id: Some("session-2".to_owned()),
            interaction_type: "review".to_owned(),
            outcome: "positive".to_owned(),
            signals_json: r#"[{"trait":"directness","delta":0.02}]"#.to_owned(),
            context_json: r#"{"surface":"mcp"}"#.to_owned(),
            notes: None,
            recorded_at: test_timestamp(2026, 3, 29, 1, 1, 0)?,
        };
        record_interaction_event(&conn, &event)?;

        let reset = AdaptationResetRecord {
            reset_id: "reset-1".to_owned(),
            agent_id: "agent.alpha".to_owned(),
            scope: ResetScope::All,
            target_key: None,
            notes: Some("Operator requested baseline restore.".to_owned()),
            recorded_at: test_timestamp(2026, 3, 29, 1, 2, 0)?,
        };

        ensure(
            record_reset(&conn, &reset)?,
            "expected reset marker to be stored",
        )?;
        ensure(
            !record_reset(&conn, &reset)?,
            "expected duplicate reset marker to be ignored",
        )?;

        let state_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM adaptation_state", [], |row| {
                row.get(0)
            })?;
        let event_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM interaction_events", [], |row| {
                row.get(0)
            })?;
        let reset_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM adaptation_resets", [], |row| {
                row.get(0)
            })?;

        ensure(
            state_count == 0,
            format!("expected cleared state, got {state_count} rows"),
        )?;
        ensure(
            event_count == 1,
            format!("expected one preserved interaction event, got {event_count}"),
        )?;
        ensure(
            reset_count == 1,
            format!("expected one reset marker, got {reset_count}"),
        )?;

        Ok(())
    }

    fn sqlite_object_exists(conn: &Connection, name: &str) -> Result<bool, rusqlite::Error> {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = ?1)",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value == 1)
    }

    fn ensure(condition: bool, message: impl Into<String>) -> Result<(), Box<dyn Error>> {
        if condition {
            Ok(())
        } else {
            Err(io::Error::other(message.into()).into())
        }
    }

    fn test_timestamp(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Result<DateTime<Utc>, Box<dyn Error>> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| io::Error::other("invalid UTC test timestamp").into())
    }
}
