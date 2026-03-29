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
    Configure(ConfigureCmd),
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
        CliCommand::Configure(cmd) => {
            let patch = cmd.trait_patch()?;
            let workspace = cmd.workspace;
            let updated = configure::update_traits(deps, workspace, patch)?;
            print_json(&updated)
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
                workspace.to_str().expect("workspace path should be utf-8"),
                "--trait",
                "verbosity",
                "0.8",
            ],
            &ApplicationConfig::new(&workspace),
            &AppDeps::default(),
        )?;

        let updated = crate::app::config::load_soul_config(&workspace)?;
        assert_eq!(updated.trait_baseline.verbosity, 0.8);
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
                workspace.to_str().expect("workspace path should be utf-8"),
                "--interaction-type",
                "review",
                "--outcome",
                "positive",
            ],
            &ApplicationConfig::new(&workspace),
            &AppDeps::default(),
        )?;

        assert!(
            WorkspacePaths::new(&workspace)
                .adaptation_db_path()
                .is_file()
        );
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
                workspace.to_str().expect("workspace path should be utf-8"),
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
        assert_eq!(reset_count, 1);

        cleanup_workspace(&workspace)?;
        Ok(())
    }

    #[test]
    fn configure_rejects_unknown_traits() {
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
        assert!(message.contains("unsupported trait `curiosity`"));
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
            .expect("system time should be after epoch")
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
}
