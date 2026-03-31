//! CLI inspect command surface.

use clap::Args;
use serde::Serialize;

use crate::{
    app::deps::SoulDependencies,
    domain::SoulError,
    services::explain::{
        InspectAdaptationProjection, InspectHeuristicProjection, InspectProvenanceProjection,
        InspectReport, InspectTraitProjection, InspectWarningProjection,
    },
};

use super::compose::{ComposeCmd, build_request};

#[derive(Debug, Clone, Args)]
pub struct InspectCmd {
    #[arg(long)]
    pub workspace: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub traits: bool,
    #[arg(long)]
    pub heuristics: bool,
    #[arg(long, alias = "adaptation")]
    pub adaptations: bool,
    #[arg(long)]
    pub warnings: bool,
    #[arg(long)]
    pub provenance: bool,
    #[arg(long = "identity-snapshot")]
    pub identity_snapshot_path: Option<String>,
    #[arg(long = "registry-verification")]
    pub registry_verification_path: Option<String>,
    #[arg(long = "registry-reputation")]
    pub registry_reputation_path: Option<String>,
    #[arg(long = "no-reputation")]
    pub no_reputation: bool,
    #[arg(long = "no-relationships")]
    pub no_relationships: bool,
    #[arg(long = "no-commitments")]
    pub no_commitments: bool,
    #[arg(long = "session-id", default_value = "cli.inspect")]
    pub session_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectProjection {
    Full,
    Traits,
    Heuristics,
    Adaptation,
    Warnings,
    Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum InspectOutput {
    Full(InspectReport),
    Traits(InspectTraitProjection),
    Heuristics(InspectHeuristicProjection),
    Adaptation(InspectAdaptationProjection),
    Warnings(InspectWarningProjection),
    Provenance(InspectProvenanceProjection),
}

impl InspectCmd {
    fn projection(&self) -> Result<InspectProjection, SoulError> {
        let selected = [
            self.traits,
            self.heuristics,
            self.adaptations,
            self.warnings,
            self.provenance,
        ]
        .into_iter()
        .filter(|selected| *selected)
        .count();

        if selected > 1 {
            return Err(SoulError::Validation(
                "inspect accepts at most one focused projection flag".into(),
            ));
        }

        Ok(if self.traits {
            InspectProjection::Traits
        } else if self.heuristics {
            InspectProjection::Heuristics
        } else if self.adaptations {
            InspectProjection::Adaptation
        } else if self.warnings {
            InspectProjection::Warnings
        } else if self.provenance {
            InspectProjection::Provenance
        } else {
            InspectProjection::Full
        })
    }
}

pub fn inspect_cmd(deps: &SoulDependencies, cmd: InspectCmd) -> Result<InspectOutput, SoulError> {
    let projection = cmd.projection()?;
    let request = build_request(
        deps,
        &ComposeCmd {
            workspace: cmd.workspace,
            json: cmd.json,
            prefix_only: false,
            identity_snapshot_path: cmd.identity_snapshot_path,
            registry_verification_path: cmd.registry_verification_path,
            registry_reputation_path: cmd.registry_reputation_path,
            no_reputation: cmd.no_reputation,
            no_relationships: cmd.no_relationships,
            no_commitments: cmd.no_commitments,
            session_id: cmd.session_id,
        },
    )?;
    let report = deps.inspect_report(request)?;

    Ok(match projection {
        InspectProjection::Full => InspectOutput::Full(report),
        InspectProjection::Traits => InspectOutput::Traits(report.traits_only()),
        InspectProjection::Heuristics => InspectOutput::Heuristics(report.heuristics_only()),
        InspectProjection::Adaptation => InspectOutput::Adaptation(report.adaptation_only()),
        InspectProjection::Warnings => InspectOutput::Warnings(report.warnings_only()),
        InspectProjection::Provenance => InspectOutput::Provenance(report.provenance_only()),
    })
}
