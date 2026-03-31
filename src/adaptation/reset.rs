use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::{
    app::config::WorkspacePaths,
    domain::SoulError,
    storage::sqlite::{self, AdaptationResetRecord, ResetScope},
};

use super::store::{StoredAdaptationState, load_stored_adaptation_state};

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveResetRequest {
    pub reset_id: String,
    pub agent_id: String,
    pub scope: ResetScope,
    pub target_key: Option<String>,
    pub notes: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum AdaptiveResetEffect {
    Duplicate,
    Cleared,
    Updated,
    RecordedWithoutState,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AdaptiveResetResult {
    pub effect: AdaptiveResetEffect,
    pub stored_state: Option<StoredAdaptationState>,
}

impl AdaptiveResetRequest {
    fn validate(&self) -> Result<(), SoulError> {
        if self.reset_id.trim().is_empty() {
            return Err(SoulError::Validation("reset_id must not be empty".into()));
        }
        if self.agent_id.trim().is_empty() {
            return Err(SoulError::Validation("agent_id must not be empty".into()));
        }
        if matches!(self.scope, ResetScope::All) && self.target_key.is_some() {
            return Err(SoulError::Validation(
                "reset target_key is only valid for targeted resets".into(),
            ));
        }
        if self
            .target_key
            .as_deref()
            .is_some_and(|target_key| target_key.trim().is_empty())
        {
            return Err(SoulError::Validation("target_key must not be empty".into()));
        }
        Ok(())
    }

    fn as_record(&self) -> AdaptationResetRecord {
        AdaptationResetRecord {
            reset_id: self.reset_id.clone(),
            agent_id: self.agent_id.clone(),
            scope: self.scope,
            target_key: self.target_key.clone(),
            notes: self.notes.clone(),
            recorded_at: self.recorded_at,
        }
    }
}

pub fn reset_adaptation_state(
    conn: &Connection,
    request: &AdaptiveResetRequest,
) -> Result<AdaptiveResetResult, SoulError> {
    request.validate()?;

    let existing = load_stored_adaptation_state(conn, &request.agent_id)?;
    let inserted = sqlite::record_reset(conn, &request.as_record())?;
    if !inserted {
        return Ok(AdaptiveResetResult {
            effect: AdaptiveResetEffect::Duplicate,
            stored_state: existing,
        });
    }

    let stored_state = load_stored_adaptation_state(conn, &request.agent_id)?;
    let effect = match (request.scope, existing.is_some()) {
        (ResetScope::All, true) => AdaptiveResetEffect::Cleared,
        (ResetScope::All, false) => AdaptiveResetEffect::RecordedWithoutState,
        (_, true) => AdaptiveResetEffect::Updated,
        (_, false) => AdaptiveResetEffect::RecordedWithoutState,
    };

    Ok(AdaptiveResetResult {
        effect,
        stored_state,
    })
}

pub fn reset_workspace_adaptation_state(
    workspace_root: impl AsRef<Path>,
    request: &AdaptiveResetRequest,
) -> Result<AdaptiveResetResult, SoulError> {
    let db_path = WorkspacePaths::new(workspace_root.as_ref().to_path_buf()).adaptation_db_path();
    let conn = sqlite::open_database(db_path)?;
    reset_adaptation_state(&conn, request)
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs, io,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use crate::{
        adaptation::{
            AdaptiveWriteRequest, persist_workspace_adaptation_write,
            read_workspace_adaptation_state,
        },
        domain::{
            CommunicationOverride, ConflictStyle, FeedbackStyle, HeuristicOverride,
            ParagraphBudget, PersonalityOverride, QuestionStyle, RegisterStyle, SoulConfig,
            UncertaintyStyle,
        },
        storage::sqlite::ResetScope,
    };

    use super::{AdaptiveResetEffect, AdaptiveResetRequest, reset_workspace_adaptation_state};

    #[test]
    fn full_reset_clears_state_without_rewriting_baseline_config() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("reset-all");
        fs::create_dir_all(&workspace)?;
        let config = sample_config();
        let config_raw = toml::to_string(&config)?;
        fs::write(workspace.join("soul.toml"), &config_raw)?;

        persist_workspace_adaptation_write(&workspace, &config, &sample_write(true)?)?;

        let result = reset_workspace_adaptation_state(
            &workspace,
            &AdaptiveResetRequest {
                reset_id: "reset-all-1".to_owned(),
                agent_id: "agent.alpha".to_owned(),
                scope: ResetScope::All,
                target_key: None,
                notes: Some("Return to baseline.".to_owned()),
                recorded_at: test_timestamp(2026, 3, 29, 2, 0, 0)?,
            },
        )?;

        assert_eq!(result.effect, AdaptiveResetEffect::Cleared);
        assert!(result.stored_state.is_none());
        assert!(read_workspace_adaptation_state(&workspace, "agent.alpha")?.is_none());
        assert_eq!(fs::read_to_string(workspace.join("soul.toml"))?, config_raw);

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn targeted_reset_clears_selected_override_and_preserves_baseline_config()
    -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("reset-targeted");
        fs::create_dir_all(&workspace)?;
        let config = sample_config();
        let config_raw = toml::to_string(&config)?;
        fs::write(workspace.join("soul.toml"), &config_raw)?;

        persist_workspace_adaptation_write(&workspace, &config, &sample_write(true)?)?;

        let result = reset_workspace_adaptation_state(
            &workspace,
            &AdaptiveResetRequest {
                reset_id: "reset-trait-1".to_owned(),
                agent_id: "agent.alpha".to_owned(),
                scope: ResetScope::Trait,
                target_key: Some("verbosity".to_owned()),
                notes: Some("Reset verbosity only.".to_owned()),
                recorded_at: test_timestamp(2026, 3, 29, 2, 5, 0)?,
            },
        )?;
        let stored = result
            .stored_state
            .ok_or_else(|| io::Error::other("missing stored state after targeted reset"))?;

        assert_eq!(result.effect, AdaptiveResetEffect::Updated);
        assert_eq!(stored.adaptation_state.trait_overrides.verbosity, 0.0);
        assert_eq!(stored.adaptation_state.trait_overrides.directness, 0.04);
        assert!(stored.adaptation_state.notes.is_empty());
        assert_eq!(stored.interaction_count, 8);
        assert_eq!(
            stored.last_reset_at,
            Some(test_timestamp(2026, 3, 29, 2, 5, 0)?)
        );
        assert_eq!(fs::read_to_string(workspace.join("soul.toml"))?, config_raw);

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn reset_without_existing_state_is_recorded_predictably() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("reset-empty");
        fs::create_dir_all(&workspace)?;

        let result = reset_workspace_adaptation_state(
            &workspace,
            &AdaptiveResetRequest {
                reset_id: "reset-empty-1".to_owned(),
                agent_id: "agent.alpha".to_owned(),
                scope: ResetScope::Communication,
                target_key: Some("paragraph_budget".to_owned()),
                notes: None,
                recorded_at: test_timestamp(2026, 3, 29, 2, 10, 0)?,
            },
        )?;

        assert_eq!(result.effect, AdaptiveResetEffect::RecordedWithoutState);
        assert!(result.stored_state.is_none());

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    fn sample_config() -> SoulConfig {
        SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        }
    }

    fn sample_write(persist: bool) -> Result<AdaptiveWriteRequest, Box<dyn Error>> {
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
            heuristic_overrides: vec![HeuristicOverride {
                heuristic_id: "ask-before-risk".to_owned(),
                priority_delta: 5,
                enabled: Some(true),
                replacement_instruction: Some("Confirm before risky actions.".to_owned()),
                note: None,
            }],
            notes: vec!["Keep answers short".to_owned()],
            evidence_window_size: 20,
            interaction_count: 8,
            last_interaction_at: Some(test_timestamp(2026, 3, 29, 1, 0, 0)?),
            updated_at: test_timestamp(2026, 3, 29, 1, 0, 0)?,
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
