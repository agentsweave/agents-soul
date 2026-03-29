pub mod compose;
pub mod configure;
pub mod explain;
pub mod inspect;
pub mod record;
pub mod reset;

use std::{ffi::OsString, io};

use clap::{Args, Parser, Subcommand};
use serde::Serialize;

use crate::{
    app::{config::ApplicationConfig, deps::AppDeps},
    domain::{PersonalityProfilePatch, SoulError},
};

#[derive(Debug, Parser)]
#[command(name = "agents-soul")]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    Compose(compose::ComposeCmd),
    Configure(ConfigureCmd),
    Inspect(inspect::InspectCmd),
    Record(record::RecordCmd),
    Reset(reset::ResetCmd),
}

#[derive(Debug, Clone, Args)]
struct ConfigureCmd {
    #[arg(long)]
    workspace: String,
    #[arg(long = "trait", num_args = 2, value_names = ["NAME", "VALUE"])]
    trait_update: Vec<String>,
}

impl ConfigureCmd {
    fn trait_patch(&self) -> Result<PersonalityProfilePatch, SoulError> {
        if self.trait_update.len() != 2 {
            return Err(SoulError::Validation(
                "configure requires exactly one --trait <NAME> <VALUE> pair".into(),
            ));
        }

        let trait_name = self.trait_update[0].trim();
        let value = self.trait_update[1].parse::<f32>().map_err(|_| {
            SoulError::Validation(format!(
                "trait value for `{trait_name}` must be a floating-point number"
            ))
        })?;

        let mut patch = PersonalityProfilePatch::default();
        match trait_name {
            "openness" => patch.openness = Some(value),
            "conscientiousness" => patch.conscientiousness = Some(value),
            "initiative" => patch.initiative = Some(value),
            "directness" => patch.directness = Some(value),
            "warmth" => patch.warmth = Some(value),
            "risk_tolerance" | "risk-tolerance" => patch.risk_tolerance = Some(value),
            "verbosity" => patch.verbosity = Some(value),
            "formality" => patch.formality = Some(value),
            _ => {
                return Err(SoulError::Validation(format!(
                    "unsupported trait `{trait_name}`"
                )));
            }
        }

        Ok(patch)
    }
}

pub fn run(config: &ApplicationConfig, deps: &AppDeps) -> Result<(), SoulError> {
    run_with_args(std::env::args_os(), config, deps)
}

fn run_with_args<I, T>(args: I, config: &ApplicationConfig, deps: &AppDeps) -> Result<(), SoulError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args).map_err(|err| SoulError::Validation(err.to_string()))?;
    execute(cli, config, deps)
}

fn execute(cli: Cli, config: &ApplicationConfig, deps: &AppDeps) -> Result<(), SoulError> {
    match cli.command {
        CliCommand::Compose(cmd) => {
            let presentation = compose::compose_cmd(deps, cmd)?;
            if let Some(rendered) = presentation.rendered {
                print_text(&rendered)
            } else {
                print_json(&presentation.output)
            }
        }
        CliCommand::Configure(cmd) => {
            let patch = cmd.trait_patch()?;
            let workspace = cmd.workspace;
            let updated = configure::update_traits(deps, workspace, patch)?;
            print_json(&updated)
        }
        CliCommand::Inspect(cmd) => {
            let output = inspect::inspect_cmd(deps, cmd)?;
            print_json(&output)
        }
        CliCommand::Record(cmd) => {
            let result = record::record_cmd(deps, config, cmd)?;
            print_debug(&result)
        }
        CliCommand::Reset(cmd) => {
            let result = reset::reset_cmd(deps, config, cmd)?;
            print_debug(&result)
        }
    }
}

fn print_json<T>(value: &T) -> Result<(), SoulError>
where
    T: Serialize,
{
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, value)
        .map_err(|error| SoulError::Internal(error.to_string()))?;
    use std::io::Write as _;
    writeln!(handle).map_err(|error| SoulError::Internal(error.to_string()))
}

fn print_debug<T>(value: &T) -> Result<(), SoulError>
where
    T: std::fmt::Debug,
{
    use std::io::Write as _;
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{value:#?}").map_err(|error| SoulError::Internal(error.to_string()))
}

fn print_text(value: &str) -> Result<(), SoulError> {
    use std::io::Write as _;
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{value}").map_err(|error| SoulError::Internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{
        app::config::WorkspacePaths,
        storage::sqlite::{self, ResetScope},
    };

    use super::*;

    #[test]
    fn configure_command_updates_workspace_trait_baseline() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("cli-configure");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        run_with_args(
            vec![
                "agents-soul",
                "configure",
                "--workspace",
                workspace.to_string_lossy().as_ref(),
                "--trait",
                "verbosity",
                "0.8",
            ],
            &ApplicationConfig::new(&workspace),
            &AppDeps::default(),
        )?;

        let updated = crate::app::config::load_soul_config(&workspace)?;
        if (updated.trait_baseline.verbosity - 0.8).abs() > f32::EPSILON {
            return Err("verbosity trait was not updated".into());
        }
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn record_command_creates_adaptation_database() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("cli-record");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        run_with_args(
            vec![
                "agents-soul",
                "record",
                "--workspace",
                workspace.to_string_lossy().as_ref(),
                "--interaction-type",
                "review",
                "--outcome",
                "positive",
            ],
            &ApplicationConfig::new(&workspace),
            &AppDeps::default(),
        )?;

        if !WorkspacePaths::new(&workspace)
            .adaptation_db_path()
            .is_file()
        {
            return Err("adaptation database was not created".into());
        }
        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn reset_command_records_reset_event() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("cli-reset");
        fs::create_dir_all(&workspace)?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;

        run_with_args(
            vec![
                "agents-soul",
                "reset",
                "--workspace",
                workspace.to_string_lossy().as_ref(),
                "--scope",
                "all",
            ],
            &ApplicationConfig::new(&workspace),
            &AppDeps::default(),
        )?;

        let conn = sqlite::open_database(WorkspacePaths::new(&workspace).adaptation_db_path())?;
        let reset_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM adaptation_resets WHERE agent_id = ?1 AND reset_scope = ?2",
            rusqlite::params!["agent.alpha", scope_name(ResetScope::All)],
            |row| row.get(0),
        )?;
        if reset_count != 1 {
            return Err(format!("expected one reset record, found {reset_count}").into());
        }

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn configure_rejects_unknown_traits() -> Result<(), Box<dyn Error>> {
        let result = run_with_args(
            vec![
                "agents-soul",
                "configure",
                "--workspace",
                "/tmp/unused",
                "--trait",
                "curiosity",
                "0.5",
            ],
            &ApplicationConfig::new("/tmp/unused"),
            &AppDeps::default(),
        );

        let message = match result {
            Err(SoulError::Validation(message)) => message,
            Err(other) => format!("expected validation error, got {other}"),
            Ok(()) => "expected validation error, got success".to_owned(),
        };
        if !message.contains("unsupported trait `curiosity`") {
            return Err(
                format!("validation message did not mention unsupported trait: {message}").into(),
            );
        }
        Ok(())
    }

    #[test]
    fn compose_command_renders_full_context_by_default() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("cli-compose");
        fs::create_dir_all(workspace.join(".soul"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        write_identity_snapshot(&workspace)?;
        write_registry_verification(&workspace)?;
        write_registry_reputation(&workspace)?;

        let command = compose::ComposeCmd {
            workspace: workspace.to_string_lossy().into_owned(),
            json: false,
            prefix_only: false,
            identity_snapshot_path: None,
            registry_verification_path: None,
            registry_reputation_path: None,
            no_reputation: false,
            no_relationships: false,
            no_commitments: false,
            session_id: "session.alpha".into(),
        };

        let presentation = compose::compose_cmd(&AppDeps::default(), command)?;
        let report = presentation
            .rendered
            .ok_or("expected rendered full-context report")?;

        match presentation.output {
            compose::ComposeOutput::Context(context) => {
                if context.profile_name != "Alpha" {
                    return Err("compose returned unexpected context profile".into());
                }
            }
            compose::ComposeOutput::Prefix(_) => {
                return Err("compose unexpectedly returned prefix-only output".into());
            }
        }
        if !report.contains("Behavioral Context Alpha") {
            return Err("compose rendered output missing report title".into());
        }

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn inspect_command_supports_traits_projection() -> Result<(), Box<dyn Error>> {
        let workspace = test_workspace("cli-inspect");
        fs::create_dir_all(workspace.join(".soul"))?;
        write_soul_config(&workspace, "agent.alpha", "Alpha")?;
        write_identity_snapshot(&workspace)?;
        write_registry_verification(&workspace)?;
        write_registry_reputation(&workspace)?;

        let output = inspect::inspect_cmd(
            &AppDeps::default(),
            inspect::InspectCmd {
                workspace: workspace.to_string_lossy().into_owned(),
                json: true,
                traits: true,
                heuristics: false,
                adaptations: false,
                warnings: false,
                provenance: false,
                identity_snapshot_path: None,
                registry_verification_path: None,
                registry_reputation_path: None,
                no_reputation: false,
                no_relationships: false,
                no_commitments: false,
                session_id: "session.alpha".into(),
            },
        )?;

        match output {
            inspect::InspectOutput::Traits(traits) => {
                if traits.entries.len() != 8 {
                    return Err(format!(
                        "expected 8 projected traits, found {}",
                        traits.entries.len()
                    )
                    .into());
                }
            }
            other => return Err(format!("expected traits projection, got {other:?}").into()),
        }

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn inspect_command_rejects_multiple_focused_views() -> Result<(), Box<dyn Error>> {
        let result = inspect::inspect_cmd(
            &AppDeps::default(),
            inspect::InspectCmd {
                workspace: "/tmp/unused".into(),
                json: true,
                traits: true,
                heuristics: true,
                adaptations: false,
                warnings: false,
                provenance: false,
                identity_snapshot_path: None,
                registry_verification_path: None,
                registry_reputation_path: None,
                no_reputation: false,
                no_relationships: false,
                no_commitments: false,
                session_id: "session.alpha".into(),
            },
        );

        let message = match result {
            Err(SoulError::Validation(message)) => message,
            Err(other) => format!("expected validation error, got {other}"),
            Ok(_) => "expected validation error, got success".to_owned(),
        };
        if !message.contains("at most one focused projection flag") {
            return Err(format!("unexpected validation message: {message}").into());
        }

        Ok(())
    }

    fn scope_name(scope: ResetScope) -> &'static str {
        match scope {
            ResetScope::All => "all",
            ResetScope::Trait => "trait",
            ResetScope::Communication => "communication",
            ResetScope::Heuristic => "heuristic",
        }
    }

    fn test_workspace(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}"))
    }

    fn cleanup_workspace(workspace: &Path) -> Result<(), Box<dyn Error>> {
        if workspace.exists() {
            fs::remove_dir_all(workspace)?;
        }
        Ok(())
    }

    fn write_soul_config(
        workspace: &Path,
        agent_id: &str,
        profile_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let config = crate::domain::SoulConfig {
            agent_id: agent_id.to_owned(),
            profile_name: profile_name.to_owned(),
            ..crate::domain::SoulConfig::default()
        };
        fs::write(workspace.join("soul.toml"), toml::to_string(&config)?)?;
        Ok(())
    }

    fn write_identity_snapshot(workspace: &Path) -> Result<(), Box<dyn Error>> {
        fs::write(
            workspace.join("session_identity_snapshot.json"),
            r#"{
  "agent_id":"agent.alpha",
  "display_name":"Alpha",
  "recovery_state":"healthy",
  "active_commitments":["review queue"],
  "durable_preferences":["lead with direct answers"],
  "relationship_markers":[{"subject":"operators","marker":"trusted"}],
  "facts":["prefers succinct status updates"],
  "warnings":[]
}"#,
        )?;
        Ok(())
    }

    fn write_registry_verification(workspace: &Path) -> Result<(), Box<dyn Error>> {
        fs::write(
            workspace.join("registry_verification.json"),
            r#"{
  "status":"active",
  "standing_level":"good",
  "reason_code":"ok"
}"#,
        )?;
        Ok(())
    }

    fn write_registry_reputation(workspace: &Path) -> Result<(), Box<dyn Error>> {
        fs::write(
            workspace.join("registry_reputation.json"),
            r#"{
  "score_total":0.91,
  "score_recent_30d":0.87,
  "context":["steady operator feedback"]
}"#,
        )?;
        Ok(())
    }
}
