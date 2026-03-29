use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    app::{
        deps::SoulDependencies,
        errors::{SoulHttpErrorResponse, SoulTransportError, map_soul_error},
    },
    domain::{ComposeRequest, PersonalityProfilePatch, SoulConfig, SoulError},
    services::{ServiceError, explain::InspectTraitProjection},
};

pub fn traits_projection(
    deps: &SoulDependencies,
    request: ComposeRequest,
) -> Result<InspectTraitProjection, ServiceError> {
    deps.inspect_report(request)
        .map(|report| report.traits_only())
}

pub fn update_traits(
    deps: &SoulDependencies,
    workspace_root: impl Into<PathBuf>,
    patch: PersonalityProfilePatch,
) -> Result<SoulConfig, ServiceError> {
    deps.update_soul_config(workspace_root, &patch.into())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateTraitsRequest {
    pub workspace_root: String,
    pub updates: BTreeMap<String, f32>,
}

impl UpdateTraitsRequest {
    fn validate(&self) -> Result<(), SoulError> {
        if self.workspace_root.trim().is_empty() {
            return Err(SoulError::Validation(
                "workspace_root must not be empty".to_owned(),
            ));
        }
        if self.updates.is_empty() {
            return Err(SoulError::Validation(
                "traits endpoint requires at least one trait update".to_owned(),
            ));
        }
        Ok(())
    }

    fn into_patch(self) -> Result<PersonalityProfilePatch, SoulError> {
        self.validate()?;

        let mut patch = PersonalityProfilePatch::default();
        for (trait_name, value) in self.updates {
            match trait_name.as_str() {
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
        }

        Ok(patch)
    }
}

pub fn handle_update_traits(
    deps: &SoulDependencies,
    request: UpdateTraitsRequest,
) -> Result<SoulConfig, SoulError> {
    let workspace_root = request.workspace_root.clone();
    let patch = request.into_patch()?;
    update_traits(deps, workspace_root, patch)
}

pub fn map_traits_error(error: &SoulError) -> SoulTransportError {
    map_soul_error(error)
}

pub fn traits_error_response(error: &SoulError) -> SoulHttpErrorResponse {
    map_traits_error(error).http_response()
}
