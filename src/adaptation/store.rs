use std::{collections::BTreeMap, path::Path};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::{
    app::config::WorkspacePaths,
    domain::{
        AdaptationState, CURRENT_SCHEMA_VERSION, CommunicationOverride, HeuristicOverride,
        PersonalityOverride, SoulConfig, SoulError,
    },
    storage::sqlite::{self, AdaptationStateRecord},
};

use super::{
    bounds::clamp_trait_delta,
    overrides::{EffectiveOverrideSet, materialize_effective_overrides},
    reducer::InteractionReduction,
};

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveWriteRequest {
    pub agent_id: String,
    pub persist: bool,
    pub trait_overrides: PersonalityOverride,
    pub communication_overrides: CommunicationOverride,
    pub heuristic_overrides: Vec<HeuristicOverride>,
    pub notes: Vec<String>,
    pub evidence_window_size: u32,
    pub interaction_count: u32,
    pub last_interaction_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveWriteEffect {
    SessionOnly,
    Inserted,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveWriteResult {
    pub effect: AdaptiveWriteEffect,
    pub stored_state: Option<StoredAdaptationState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredAdaptationState {
    pub agent_id: String,
    pub adaptation_state: AdaptationState,
    pub interaction_count: u32,
    pub last_interaction_at: Option<DateTime<Utc>>,
    pub last_reset_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl AdaptiveWriteRequest {
    pub fn from_reduction(
        agent_id: impl Into<String>,
        persist: bool,
        updated_at: DateTime<Utc>,
        reduction: &InteractionReduction,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            persist,
            trait_overrides: reduction.adaptation_state.trait_overrides.clone(),
            communication_overrides: reduction.adaptation_state.communication_overrides.clone(),
            heuristic_overrides: reduction.adaptation_state.heuristic_overrides.clone(),
            notes: reduction.adaptation_state.notes.clone(),
            evidence_window_size: reduction.adaptation_state.evidence_window_size,
            interaction_count: reduction.interaction_count,
            last_interaction_at: reduction.last_interaction_at,
            updated_at,
        }
    }

    fn validate(&self) -> Result<(), SoulError> {
        if self.agent_id.trim().is_empty() {
            return Err(SoulError::Validation("agent_id must not be empty".into()));
        }
        if self.evidence_window_size == 0 {
            return Err(SoulError::Validation(
                "adaptive writes require evidence_window_size > 0".into(),
            ));
        }
        for override_rule in &self.heuristic_overrides {
            if override_rule.heuristic_id.trim().is_empty() {
                return Err(SoulError::Validation(
                    "adaptation.heuristic_overrides[].heuristic_id must not be empty".into(),
                ));
            }
        }
        Ok(())
    }

    fn normalized_state(
        &self,
        config: &SoulConfig,
        last_reset_at: Option<DateTime<Utc>>,
    ) -> Result<StoredAdaptationState, SoulError> {
        self.validate()?;

        let notes = normalize_notes(self.notes.clone());
        let mut heuristic_overrides =
            normalize_heuristic_overrides(self.heuristic_overrides.clone());
        heuristic_overrides.truncate(config.limits.max_adaptive_rules);

        Ok(StoredAdaptationState {
            agent_id: self.agent_id.clone(),
            adaptation_state: AdaptationState {
                schema_version: CURRENT_SCHEMA_VERSION,
                last_updated_at: Some(self.updated_at),
                trait_overrides: clamp_personality_override(
                    &self.trait_overrides,
                    config.limits.max_trait_drift,
                ),
                communication_overrides: self.communication_overrides.clone(),
                heuristic_overrides,
                evidence_window_size: self.evidence_window_size,
                notes,
            },
            interaction_count: self.interaction_count,
            last_interaction_at: self.last_interaction_at,
            last_reset_at,
            updated_at: self.updated_at,
        })
    }
}

impl StoredAdaptationState {
    pub fn to_record(&self) -> Result<AdaptationStateRecord, SoulError> {
        Ok(AdaptationStateRecord {
            agent_id: self.agent_id.clone(),
            trait_overrides_json: serialize_json(&self.adaptation_state.trait_overrides)?,
            communication_overrides_json: serialize_json(
                &self.adaptation_state.communication_overrides,
            )?,
            heuristic_overrides_json: serialize_json(&self.adaptation_state.heuristic_overrides)?,
            notes_json: serialize_json(&self.adaptation_state.notes)?,
            evidence_window_size: self.adaptation_state.evidence_window_size,
            interaction_count: self.interaction_count,
            last_interaction_at: self.last_interaction_at,
            last_reset_at: self.last_reset_at,
            updated_at: self.updated_at,
        })
    }

    pub fn equivalent_payload(&self, other: &Self) -> bool {
        self.agent_id == other.agent_id
            && self.adaptation_state.trait_overrides == other.adaptation_state.trait_overrides
            && self.adaptation_state.communication_overrides
                == other.adaptation_state.communication_overrides
            && self.adaptation_state.heuristic_overrides
                == other.adaptation_state.heuristic_overrides
            && self.adaptation_state.evidence_window_size
                == other.adaptation_state.evidence_window_size
            && self.adaptation_state.notes == other.adaptation_state.notes
            && self.interaction_count == other.interaction_count
            && self.last_reset_at == other.last_reset_at
    }
}

pub fn read_workspace_adaptation_state(
    workspace_root: impl AsRef<Path>,
    agent_id: &str,
) -> Result<Option<StoredAdaptationState>, SoulError> {
    let db_path = WorkspacePaths::new(workspace_root.as_ref().to_path_buf()).adaptation_db_path();
    if !db_path.is_file() {
        return Ok(None);
    }

    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| SoulError::Storage(error.to_string()))?;
    load_stored_adaptation_state(&conn, agent_id)
}

pub fn load_stored_adaptation_state(
    conn: &Connection,
    agent_id: &str,
) -> Result<Option<StoredAdaptationState>, SoulError> {
    conn.query_row(
        r#"
SELECT
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
            let updated_at = parse_required_timestamp(&row.get::<_, String>(8)?)?;
            let last_interaction_at = row
                .get::<_, Option<String>>(6)?
                .map(|raw| parse_required_timestamp(&raw))
                .transpose()?;
            let last_reset_at = row
                .get::<_, Option<String>>(7)?
                .map(|raw| parse_required_timestamp(&raw))
                .transpose()?;

            Ok(StoredAdaptationState {
                agent_id: agent_id.to_owned(),
                adaptation_state: AdaptationState {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    last_updated_at: Some(updated_at),
                    trait_overrides: deserialize_json(row.get_ref(0)?.as_str()?)?,
                    communication_overrides: deserialize_json(row.get_ref(1)?.as_str()?)?,
                    heuristic_overrides: normalize_heuristic_overrides(deserialize_json(
                        row.get_ref(2)?.as_str()?,
                    )?),
                    evidence_window_size: read_u32_column(
                        row.get::<_, i64>(4)?,
                        "evidence_window_size",
                    )?,
                    notes: normalize_notes(deserialize_json(row.get_ref(3)?.as_str()?)?),
                },
                interaction_count: read_u32_column(row.get::<_, i64>(5)?, "interaction_count")?,
                last_interaction_at,
                last_reset_at,
                updated_at,
            })
        },
    )
    .optional()
    .map_err(|error| SoulError::Storage(error.to_string()))
}

pub fn load_effective_adaptation_state(
    conn: &Connection,
    config: &SoulConfig,
    agent_id: &str,
) -> Result<EffectiveOverrideSet, SoulError> {
    let stored = load_stored_adaptation_state(conn, agent_id)?;
    Ok(materialize_effective_overrides(config, stored.as_ref()))
}

pub fn read_workspace_effective_overrides(
    workspace_root: impl AsRef<Path>,
    config: &SoulConfig,
    agent_id: &str,
) -> Result<EffectiveOverrideSet, SoulError> {
    let stored = read_workspace_adaptation_state(workspace_root, agent_id)?;
    Ok(materialize_effective_overrides(config, stored.as_ref()))
}

pub fn persist_adaptation_write(
    conn: &Connection,
    config: &SoulConfig,
    request: &AdaptiveWriteRequest,
) -> Result<AdaptiveWriteResult, SoulError> {
    if !request.persist {
        request.validate()?;
        return Ok(AdaptiveWriteResult {
            effect: AdaptiveWriteEffect::SessionOnly,
            stored_state: None,
        });
    }

    let existing = load_stored_adaptation_state(conn, &request.agent_id)?;
    let candidate = request.normalized_state(
        config,
        existing.as_ref().and_then(|state| state.last_reset_at),
    )?;

    if existing
        .as_ref()
        .is_some_and(|stored: &StoredAdaptationState| stored.equivalent_payload(&candidate))
    {
        return Ok(AdaptiveWriteResult {
            effect: AdaptiveWriteEffect::Unchanged,
            stored_state: existing,
        });
    }

    sqlite::upsert_adaptation_state(conn, &candidate.to_record()?)?;

    Ok(AdaptiveWriteResult {
        effect: if existing.is_some() {
            AdaptiveWriteEffect::Updated
        } else {
            AdaptiveWriteEffect::Inserted
        },
        stored_state: Some(candidate),
    })
}

pub fn persist_workspace_adaptation_write(
    workspace_root: impl AsRef<Path>,
    config: &SoulConfig,
    request: &AdaptiveWriteRequest,
) -> Result<AdaptiveWriteResult, SoulError> {
    let db_path = WorkspacePaths::new(workspace_root.as_ref().to_path_buf()).adaptation_db_path();
    let conn = sqlite::open_database(db_path)?;
    persist_adaptation_write(&conn, config, request)
}

fn normalize_notes(mut notes: Vec<String>) -> Vec<String> {
    notes.retain(|note| !note.trim().is_empty());
    notes.sort();
    notes.dedup();
    notes
}

fn normalize_heuristic_overrides(overrides: Vec<HeuristicOverride>) -> Vec<HeuristicOverride> {
    let mut by_id = BTreeMap::new();
    for override_rule in overrides {
        if override_rule.heuristic_id.trim().is_empty() {
            continue;
        }
        by_id.insert(override_rule.heuristic_id.clone(), override_rule);
    }
    by_id.into_values().collect()
}

fn clamp_personality_override(
    override_set: &PersonalityOverride,
    max_trait_drift: f32,
) -> PersonalityOverride {
    PersonalityOverride {
        openness: clamp_trait_delta(override_set.openness, max_trait_drift),
        conscientiousness: clamp_trait_delta(override_set.conscientiousness, max_trait_drift),
        initiative: clamp_trait_delta(override_set.initiative, max_trait_drift),
        directness: clamp_trait_delta(override_set.directness, max_trait_drift),
        warmth: clamp_trait_delta(override_set.warmth, max_trait_drift),
        risk_tolerance: clamp_trait_delta(override_set.risk_tolerance, max_trait_drift),
        verbosity: clamp_trait_delta(override_set.verbosity, max_trait_drift),
        formality: clamp_trait_delta(override_set.formality, max_trait_drift),
    }
}

fn deserialize_json<T>(raw: &str) -> Result<T, rusqlite::Error>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn serialize_json<T>(value: &T) -> Result<String, SoulError>
where
    T: serde::Serialize,
{
    serde_json::to_string(value).map_err(|error| SoulError::Storage(error.to_string()))
}

fn parse_required_timestamp(raw: &str) -> Result<DateTime<Utc>, rusqlite::Error> {
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn read_u32_column(value: i64, field: &str) -> Result<u32, rusqlite::Error> {
    value.try_into().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(std::io::Error::other(format!(
                "{field} must fit into u32, got {value}",
            ))),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AdaptiveWriteEffect, AdaptiveWriteRequest, load_effective_adaptation_state,
        load_stored_adaptation_state, persist_adaptation_write, persist_workspace_adaptation_write,
        read_workspace_effective_overrides,
    };
    use crate::{
        app::config::WorkspacePaths,
        domain::{
            CommunicationOverride, ConflictStyle, FeedbackStyle, HeuristicOverride,
            ParagraphBudget, PersonalityOverride, QuestionStyle, RegisterStyle, UncertaintyStyle,
        },
        storage::sqlite::initialize_database,
    };
    use chrono::{TimeZone, Utc};
    use rusqlite::Connection;
    use std::{
        error::Error,
        fs, io,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn persist_false_leaves_storage_untouched() -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;
        initialize_database(&conn)?;

        let config = sample_config();
        let result = persist_adaptation_write(&conn, &config, &sample_write(false, 0)?)?;

        assert_eq!(result.effect, AdaptiveWriteEffect::SessionOnly);
        assert!(result.stored_state.is_none());
        assert!(load_stored_adaptation_state(&conn, "agent.alpha")?.is_none());
        Ok(())
    }

    #[test]
    fn repeated_equivalent_writes_are_idempotent() -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;
        initialize_database(&conn)?;

        let first = sample_write(true, 0)?;
        let second = sample_write(true, 0)?;
        let config = sample_config();

        let inserted = persist_adaptation_write(&conn, &config, &first)?;
        let unchanged = persist_adaptation_write(&conn, &config, &second)?;
        let stored = load_stored_adaptation_state(&conn, "agent.alpha")?
            .ok_or_else(|| io::Error::other("missing stored adaptation state"))?;

        assert_eq!(inserted.effect, AdaptiveWriteEffect::Inserted);
        assert_eq!(unchanged.effect, AdaptiveWriteEffect::Unchanged);
        assert_eq!(stored.updated_at, first.updated_at);
        assert_eq!(
            stored.adaptation_state.notes,
            vec!["Keep answers short".to_owned()]
        );
        assert_eq!(stored.adaptation_state.heuristic_overrides.len(), 2);
        assert_eq!(
            stored.adaptation_state.heuristic_overrides[0].heuristic_id,
            "ask-before-risk"
        );
        assert_eq!(
            stored.adaptation_state.heuristic_overrides[1].heuristic_id,
            "cite-sources"
        );
        Ok(())
    }

    #[test]
    fn durable_write_updates_existing_state_and_preserves_reset_metadata()
    -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;
        initialize_database(&conn)?;

        let first = sample_write(true, 0)?;
        let second = AdaptiveWriteRequest {
            updated_at: test_timestamp(2026, 3, 29, 1, 10, 0)?,
            interaction_count: 12,
            notes: vec!["New signal".to_owned()],
            ..sample_write(true, 10)?
        };
        let config = sample_config();

        let inserted = persist_adaptation_write(&conn, &config, &first)?;
        let mut seeded = inserted
            .stored_state
            .ok_or_else(|| io::Error::other("missing stored state after insert"))?;
        seeded.last_reset_at = Some(test_timestamp(2026, 3, 29, 1, 5, 0)?);
        crate::storage::sqlite::upsert_adaptation_state(&conn, &seeded.to_record()?)?;

        let updated = persist_adaptation_write(&conn, &config, &second)?;
        let stored = load_stored_adaptation_state(&conn, "agent.alpha")?
            .ok_or_else(|| io::Error::other("missing stored adaptation state"))?;

        assert_eq!(updated.effect, AdaptiveWriteEffect::Updated);
        assert_eq!(stored.last_reset_at, seeded.last_reset_at);
        assert_eq!(stored.updated_at, second.updated_at);
        assert_eq!(stored.interaction_count, 12);
        assert_eq!(stored.adaptation_state.notes, vec!["New signal".to_owned()]);
        Ok(())
    }

    #[test]
    fn workspace_read_returns_none_when_database_is_missing() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("missing-db");

        let stored = super::read_workspace_adaptation_state(&workspace, "agent.alpha")?;

        assert!(stored.is_none());
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn workspace_read_loads_existing_stored_state() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("workspace-read");
        fs::create_dir_all(&workspace)?;
        let db_path = WorkspacePaths::new(&workspace).adaptation_db_path();
        let conn = crate::storage::sqlite::open_database(&db_path)?;

        let write = sample_write(true, 0)?;
        let config = sample_config();
        persist_adaptation_write(&conn, &config, &write)?;

        let stored = super::read_workspace_adaptation_state(&workspace, "agent.alpha")?
            .ok_or_else(|| io::Error::other("missing stored adaptation state"))?;

        assert_eq!(stored.agent_id, "agent.alpha");
        assert_eq!(stored.interaction_count, 8);
        assert_eq!(
            stored.adaptation_state.notes,
            vec!["Keep answers short".to_owned()]
        );
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn effective_state_load_clamps_drift_and_rule_count() -> Result<(), Box<dyn Error>> {
        let conn = Connection::open_in_memory()?;
        initialize_database(&conn)?;
        let mut config = sample_config();
        config.limits.max_trait_drift = 0.05;
        config.limits.max_adaptive_rules = 1;
        config.adaptation.min_interactions_for_adapt = 1;

        let write = AdaptiveWriteRequest {
            trait_overrides: PersonalityOverride {
                verbosity: 0.20,
                warmth: -0.20,
                ..PersonalityOverride::default()
            },
            heuristic_overrides: vec![
                HeuristicOverride {
                    heuristic_id: "zeta".to_owned(),
                    priority_delta: 2,
                    enabled: Some(true),
                    replacement_instruction: None,
                    note: None,
                },
                HeuristicOverride {
                    heuristic_id: "alpha".to_owned(),
                    priority_delta: 1,
                    enabled: Some(true),
                    replacement_instruction: None,
                    note: None,
                },
            ],
            ..sample_write(true, 0)?
        };

        persist_adaptation_write(&conn, &config, &write)?;
        let effective = load_effective_adaptation_state(&conn, &config, "agent.alpha")?;

        assert_eq!(effective.adaptation_state.trait_overrides.verbosity, 0.05);
        assert_eq!(effective.adaptation_state.trait_overrides.warmth, -0.05);
        assert_eq!(effective.adaptation_state.heuristic_overrides.len(), 1);
        assert_eq!(
            effective.adaptation_state.heuristic_overrides[0].heuristic_id,
            "alpha"
        );
        Ok(())
    }

    #[test]
    fn workspace_persist_and_effective_read_share_bounded_state() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("workspace-effective");
        fs::create_dir_all(&workspace)?;
        let mut config = sample_config();
        config.limits.max_trait_drift = 0.04;
        config.limits.max_adaptive_rules = 1;
        config.adaptation.min_interactions_for_adapt = 1;

        let write = AdaptiveWriteRequest {
            trait_overrides: PersonalityOverride {
                verbosity: -0.20,
                ..PersonalityOverride::default()
            },
            heuristic_overrides: vec![
                HeuristicOverride {
                    heuristic_id: "zeta".to_owned(),
                    priority_delta: 2,
                    enabled: Some(true),
                    replacement_instruction: None,
                    note: None,
                },
                HeuristicOverride {
                    heuristic_id: "alpha".to_owned(),
                    priority_delta: 1,
                    enabled: Some(true),
                    replacement_instruction: None,
                    note: None,
                },
            ],
            ..sample_write(true, 0)?
        };

        persist_workspace_adaptation_write(&workspace, &config, &write)?;
        let stored = super::read_workspace_adaptation_state(&workspace, "agent.alpha")?
            .ok_or_else(|| io::Error::other("missing stored adaptation state"))?;
        let effective = read_workspace_effective_overrides(&workspace, &config, "agent.alpha")?;

        assert_eq!(stored.adaptation_state.trait_overrides.verbosity, -0.04);
        assert_eq!(stored.adaptation_state.heuristic_overrides.len(), 1);
        assert_eq!(effective.adaptation_state.trait_overrides.verbosity, -0.04);
        assert_eq!(effective.adaptation_state.heuristic_overrides.len(), 1);

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    fn sample_config() -> crate::domain::SoulConfig {
        crate::domain::SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..crate::domain::SoulConfig::default()
        }
    }

    fn sample_write(
        persist: bool,
        minute_offset: u32,
    ) -> Result<AdaptiveWriteRequest, Box<dyn Error>> {
        Ok(AdaptiveWriteRequest {
            agent_id: "agent.alpha".to_owned(),
            persist,
            trait_overrides: PersonalityOverride {
                verbosity: -0.06,
                directness: 0.04,
                ..PersonalityOverride::default()
            },
            communication_overrides: CommunicationOverride {
                default_register: Some(RegisterStyle::ProfessionalDirect),
                paragraph_budget: Some(ParagraphBudget::Short),
                question_style: Some(QuestionStyle::QuestionFreeUnlessBlocked),
                uncertainty_style: Some(UncertaintyStyle::ExplicitAndBounded),
                feedback_style: Some(FeedbackStyle::Frank),
                conflict_style: Some(ConflictStyle::FirmRespectful),
            },
            heuristic_overrides: vec![
                HeuristicOverride {
                    heuristic_id: "cite-sources".to_owned(),
                    priority_delta: 3,
                    enabled: Some(true),
                    replacement_instruction: None,
                    note: Some("Positive evidence".to_owned()),
                },
                HeuristicOverride {
                    heuristic_id: "ask-before-risk".to_owned(),
                    priority_delta: 5,
                    enabled: Some(true),
                    replacement_instruction: Some("Confirm before risky actions.".to_owned()),
                    note: None,
                },
                HeuristicOverride {
                    heuristic_id: "cite-sources".to_owned(),
                    priority_delta: 3,
                    enabled: Some(true),
                    replacement_instruction: None,
                    note: Some("Positive evidence".to_owned()),
                },
            ],
            notes: vec![
                "Keep answers short".to_owned(),
                "Keep answers short".to_owned(),
                "".to_owned(),
            ],
            evidence_window_size: 20,
            interaction_count: 8,
            last_interaction_at: Some(test_timestamp(2026, 3, 29, 1, minute_offset, 0)?),
            updated_at: test_timestamp(2026, 3, 29, 1, minute_offset, 0)?,
        })
    }

    fn test_timestamp(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Result<chrono::DateTime<Utc>, Box<dyn Error>> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| io::Error::other("invalid UTC test timestamp").into())
    }

    fn test_workspace(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}"))
    }

    fn cleanup_workspace(workspace: &PathBuf) -> Result<(), Box<dyn Error>> {
        if workspace.exists() {
            fs::remove_dir_all(workspace)?;
        }
        Ok(())
    }
}
