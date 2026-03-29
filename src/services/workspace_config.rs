//! Canonical workspace config authoring service.
//!
//! Persistence rewrites the full `soul.toml` from a validated `SoulConfig` using
//! canonical TOML formatting and heuristic ordering. Comments and original table
//! layout are not preserved.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    app::config::{WorkspacePaths, load_soul_config},
    domain::config_patch::canonicalize_heuristics,
    domain::{SoulConfig, SoulConfigPatch, SoulError},
};

#[derive(Debug, Clone, Default)]
pub struct WorkspaceConfigService;

impl WorkspaceConfigService {
    pub fn patch_workspace(
        &self,
        workspace_root: impl Into<PathBuf>,
        patch: &SoulConfigPatch,
    ) -> Result<SoulConfig, SoulError> {
        let workspace_root = workspace_root.into();
        let paths = WorkspacePaths::new(workspace_root);
        let existing = load_soul_config(paths.root().to_path_buf())?;
        let updated = patch.apply(&existing)?;
        let rendered = render_canonical_toml(&updated)?;

        atomic_write(paths.config_path().as_path(), rendered.as_bytes())?;
        Ok(updated)
    }
}

pub fn render_canonical_toml(config: &SoulConfig) -> Result<String, SoulError> {
    let mut canonical = config.clone().finalize()?;
    canonicalize_heuristics(&mut canonical.decision_heuristics);
    let mut raw = toml::to_string_pretty(&canonical)
        .map_err(|error| SoulError::InvalidConfig(error.to_string()))?;

    if !raw.ends_with('\n') {
        raw.push('\n');
    }

    Ok(raw)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SoulError> {
    let Some(parent) = path.parent() else {
        return Err(SoulError::Storage(format!(
            "cannot determine parent directory for `{}`",
            path.display()
        )));
    };

    let temp_path = temp_config_path(path)?;
    let result = write_temp_and_swap(path, &temp_path, parent, bytes);
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn write_temp_and_swap(
    destination: &Path,
    temp_path: &Path,
    parent: &Path,
    bytes: &[u8],
) -> Result<(), SoulError> {
    let mut temp_file = create_temp_file(temp_path)?;
    temp_file.write_all(bytes).map_err(|error| {
        SoulError::Storage(format!(
            "failed to write `{}`: {error}",
            temp_path.display()
        ))
    })?;
    temp_file.sync_all().map_err(|error| {
        SoulError::Storage(format!("failed to sync `{}`: {error}", temp_path.display()))
    })?;
    drop(temp_file);

    fs::rename(temp_path, destination).map_err(|error| {
        SoulError::Storage(format!(
            "failed to atomically replace `{}`: {error}",
            destination.display()
        ))
    })?;

    sync_directory(parent)?;
    Ok(())
}

fn create_temp_file(path: &Path) -> Result<File, SoulError> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            SoulError::Storage(format!(
                "failed to create temp config `{}`: {error}",
                path.display()
            ))
        })
}

fn temp_config_path(path: &Path) -> Result<PathBuf, SoulError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SoulError::Storage(format!("clock error while writing config: {error}")))?
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            SoulError::Storage(format!("invalid config file name `{}`", path.display()))
        })?;
    Ok(path.with_file_name(format!(".{file_name}.{nanos}.{}.tmp", std::process::id())))
}

fn sync_directory(path: &Path) -> Result<(), SoulError> {
    File::open(path)
        .and_then(|dir| dir.sync_all())
        .map_err(|error| {
            SoulError::Storage(format!(
                "failed to sync config directory `{}`: {error}",
                path.display()
            ))
        })
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use crate::{
        app::config::load_soul_config,
        domain::{
            CommunicationStylePatch, ConflictStyle, DecisionHeuristic, DecisionHeuristicPatch,
            ParagraphBudget, PersonalityProfilePatch, QuestionStyle, RegisterStyle, SoulConfig,
            SoulConfigPatch,
        },
    };

    use super::{WorkspaceConfigService, render_canonical_toml};

    #[test]
    fn patch_updates_traits_style_and_heuristics() -> Result<(), Box<dyn std::error::Error>> {
        let config = sample_config();
        let patch = SoulConfigPatch {
            trait_baseline: PersonalityProfilePatch {
                openness: Some(0.63),
                directness: Some(0.88),
                ..PersonalityProfilePatch::default()
            },
            communication_style: CommunicationStylePatch {
                default_register: Some(RegisterStyle::Advisory),
                paragraph_budget: Some(ParagraphBudget::Long),
                question_style: Some(QuestionStyle::QuestionFreeUnlessBlocked),
                conflict_style: Some(ConflictStyle::OperatorEscalation),
                ..CommunicationStylePatch::default()
            },
            decision_heuristics: DecisionHeuristicPatch {
                upsert: vec![
                    DecisionHeuristic {
                        heuristic_id: "zeta".to_owned(),
                        title: "Zeta".to_owned(),
                        priority: 3,
                        trigger: "escalation".to_owned(),
                        instruction: "Escalate immediately.".to_owned(),
                        enabled: true,
                        ..DecisionHeuristic::default()
                    },
                    DecisionHeuristic {
                        heuristic_id: "alpha".to_owned(),
                        title: "Alpha v2".to_owned(),
                        priority: 9,
                        trigger: "review".to_owned(),
                        instruction: "Review with evidence first.".to_owned(),
                        enabled: true,
                        ..DecisionHeuristic::default()
                    },
                ],
                remove: vec!["beta".to_owned()],
                ..DecisionHeuristicPatch::default()
            },
        };

        let updated = patch.apply(&config)?;

        assert_eq!(updated.trait_baseline.openness, 0.63);
        assert_eq!(updated.trait_baseline.directness, 0.88);
        assert_eq!(
            updated.communication_style.default_register,
            RegisterStyle::Advisory
        );
        assert_eq!(
            updated.communication_style.paragraph_budget,
            ParagraphBudget::Long
        );
        assert_eq!(
            updated.communication_style.question_style,
            QuestionStyle::QuestionFreeUnlessBlocked
        );
        assert_eq!(
            updated.communication_style.conflict_style,
            ConflictStyle::OperatorEscalation
        );
        assert_eq!(
            updated
                .decision_heuristics
                .iter()
                .map(|heuristic| heuristic.heuristic_id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
        assert_eq!(updated.decision_heuristics[0].title, "Alpha v2");

        Ok(())
    }

    #[test]
    fn render_canonical_toml_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = sample_config();
        config.decision_heuristics = vec![
            DecisionHeuristic {
                heuristic_id: "beta".to_owned(),
                title: "Beta".to_owned(),
                priority: 2,
                trigger: "follow-up".to_owned(),
                instruction: "Follow up explicitly.".to_owned(),
                enabled: true,
                ..DecisionHeuristic::default()
            },
            DecisionHeuristic {
                heuristic_id: "alpha".to_owned(),
                title: "Alpha".to_owned(),
                priority: 8,
                trigger: "review".to_owned(),
                instruction: "Lead with evidence.".to_owned(),
                enabled: true,
                ..DecisionHeuristic::default()
            },
        ];

        let first = render_canonical_toml(&config)?;
        let second = render_canonical_toml(&config)?;

        assert_eq!(first, second);
        assert!(first.ends_with('\n'));
        assert!(
            first.find("heuristic_id = \"alpha\"").unwrap()
                < first.find("heuristic_id = \"beta\"").unwrap()
        );

        Ok(())
    }

    #[test]
    fn invalid_patch_does_not_corrupt_existing_config() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = temp_workspace("invalid-patch");
        fs::create_dir_all(&workspace)?;
        let config_path = workspace.join("soul.toml");
        let initial = render_canonical_toml(&sample_config())?;
        fs::write(&config_path, &initial)?;

        let patch = SoulConfigPatch {
            trait_baseline: PersonalityProfilePatch {
                warmth: Some(1.4),
                ..PersonalityProfilePatch::default()
            },
            ..SoulConfigPatch::default()
        };

        let service = WorkspaceConfigService;
        let error = service.patch_workspace(&workspace, &patch).unwrap_err();
        assert!(matches!(
            error,
            crate::domain::SoulError::InvalidConfig(ref message)
                if message.contains("trait `warmth` must be within 0.0..=1.0")
        ));
        assert_eq!(fs::read_to_string(&config_path)?, initial);
        let loaded = load_soul_config(&workspace)?;
        assert_eq!(loaded, sample_config());

        Ok(())
    }

    fn sample_config() -> SoulConfig {
        let mut config = SoulConfig {
            agent_id: "alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.sources.identity_workspace = "/tmp/identity".to_owned();
        config.sources.registry_url = "https://registry.example.test".to_owned();
        config.sources.registry_agent_id = "alpha".to_owned();
        config.decision_heuristics = vec![
            DecisionHeuristic {
                heuristic_id: "alpha".to_owned(),
                title: "Alpha".to_owned(),
                priority: 8,
                trigger: "review".to_owned(),
                instruction: "Lead with evidence.".to_owned(),
                enabled: true,
                ..DecisionHeuristic::default()
            },
            DecisionHeuristic {
                heuristic_id: "beta".to_owned(),
                title: "Beta".to_owned(),
                priority: 1,
                trigger: "follow-up".to_owned(),
                instruction: "Follow up explicitly.".to_owned(),
                enabled: true,
                ..DecisionHeuristic::default()
            },
        ];
        config
    }

    fn temp_workspace(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("valid system time")
            .as_nanos();
        env::temp_dir().join(format!(
            "agents-soul-workspace-config-{label}-{}-{nanos}",
            std::process::id()
        ))
    }
}
