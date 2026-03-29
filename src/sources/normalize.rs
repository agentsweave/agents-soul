use std::cmp::Reverse;

use crate::domain::{
    BehaviorInputs, ComposeMode, ComposeRequest, InputProvenance, InputSourceKind,
    NormalizedInputs, RecoveryState, RegistryStatus, RelationshipMarker, SoulError,
    WarningSeverity,
};
use crate::sources::identity::agent_mismatch_warning;

pub fn normalize_inputs(
    request: &ComposeRequest,
    mut inputs: BehaviorInputs,
) -> Result<NormalizedInputs, SoulError> {
    request.validate()?;
    inputs.soul_config.validate()?;

    inputs
        .soul_config
        .decision_heuristics
        .sort_by_key(|heuristic| (Reverse(heuristic.priority), heuristic.heuristic_id.clone()));
    inputs
        .soul_config
        .decision_heuristics
        .dedup_by(|left, right| left.heuristic_id == right.heuristic_id);

    inputs.adaptation_state.notes.sort();
    inputs.adaptation_state.notes.dedup();
    inputs
        .adaptation_state
        .heuristic_overrides
        .sort_by(|left, right| left.heuristic_id.cmp(&right.heuristic_id));
    inputs
        .adaptation_state
        .heuristic_overrides
        .dedup_by(|left, right| left.heuristic_id == right.heuristic_id);

    inputs.reader_warnings.sort_by(|left, right| {
        (
            severity_rank(left.severity),
            left.code.as_str(),
            left.message.as_str(),
        )
            .cmp(&(
                severity_rank(right.severity),
                right.code.as_str(),
                right.message.as_str(),
            ))
    });
    inputs
        .reader_warnings
        .dedup_by(|left, right| left.code == right.code && left.message == right.message);

    let mut reader_warnings = inputs.reader_warnings;

    let identity_snapshot = inputs.identity_snapshot.and_then(|mut snapshot| {
        if snapshot.agent_id != request.agent_id {
            reader_warnings.push(agent_mismatch_warning(
                &request.agent_id,
                &snapshot.agent_id,
            ));
            return None;
        }

        snapshot.active_commitments.sort();
        snapshot.active_commitments.dedup();
        snapshot.durable_preferences.sort();
        snapshot.durable_preferences.dedup();
        sort_relationships(&mut snapshot.relationship_markers);
        snapshot.facts.sort();
        snapshot.facts.dedup();
        snapshot.warnings.sort_by(|left, right| {
            (
                severity_rank(left.severity),
                left.code.as_str(),
                left.message.as_str(),
            )
                .cmp(&(
                    severity_rank(right.severity),
                    right.code.as_str(),
                    right.message.as_str(),
                ))
        });
        snapshot.warnings.dedup_by(|left, right| {
            left.severity == right.severity
                && left.code == right.code
                && left.message == right.message
        });

        if !request.include_commitments {
            snapshot.active_commitments.clear();
        }
        if !request.include_relationships {
            snapshot.relationship_markers.clear();
        }

        Some(snapshot)
    });

    let verification_result = inputs.verification_result;
    let verification_provenance = if verification_result.is_some() {
        inputs.verification_provenance
    } else {
        unavailable_if_missing(
            inputs.verification_provenance,
            "registry verification unavailable after source selection",
        )
    };
    let reputation_summary = if request.include_reputation && verification_result.is_some() {
        inputs.reputation_summary.map(|mut summary| {
            summary.context.sort();
            summary.context.dedup();
            summary
        })
    } else {
        None
    };
    let identity_provenance = if identity_snapshot.is_some() {
        inputs.identity_provenance
    } else {
        unavailable_if_missing(
            inputs.identity_provenance,
            "identity snapshot unavailable after source selection",
        )
    };
    let reputation_provenance = if reputation_summary.is_some() {
        inputs.reputation_provenance
    } else if !request.include_reputation {
        InputProvenance::unavailable("reputation disabled by request")
    } else {
        unavailable_if_missing(
            inputs.reputation_provenance,
            "registry reputation unavailable after source selection",
        )
    };

    let identity_recovery_state = identity_snapshot
        .as_ref()
        .map(|snapshot| snapshot.recovery_state)
        .or(inputs.identity_recovery_state);

    let compose_mode_hint = compose_mode_hint(
        verification_result
            .as_ref()
            .map(|verification| verification.status),
        identity_recovery_state,
        inputs.soul_config.limits.offline_registry_behavior,
    );

    Ok(NormalizedInputs {
        schema_version: inputs.schema_version,
        request: request.clone(),
        agent_id: request.agent_id.clone(),
        profile_name: inputs.soul_config.profile_name.clone(),
        compose_mode_hint: Some(compose_mode_hint),
        identity_snapshot,
        identity_recovery_state,
        identity_provenance,
        verification_result,
        verification_provenance,
        reputation_summary,
        reputation_provenance,
        soul_config: inputs.soul_config,
        adaptation_state: inputs.adaptation_state,
        reader_warnings,
        generated_at: inputs.generated_at,
    })
}

fn sort_relationships(relationships: &mut [RelationshipMarker]) {
    relationships.sort_by(|left, right| {
        (
            left.subject.as_str(),
            left.marker.as_str(),
            left.note.as_deref().unwrap_or_default(),
        )
            .cmp(&(
                right.subject.as_str(),
                right.marker.as_str(),
                right.note.as_deref().unwrap_or_default(),
            ))
    });
}

fn severity_rank(severity: WarningSeverity) -> u8 {
    match severity {
        WarningSeverity::Info => 0,
        WarningSeverity::Caution => 1,
        WarningSeverity::Important => 2,
        WarningSeverity::Severe => 3,
    }
}

fn unavailable_if_missing(provenance: InputProvenance, fallback_detail: &str) -> InputProvenance {
    match provenance.source {
        InputSourceKind::Unavailable => provenance,
        _ => InputProvenance::unavailable(fallback_detail),
    }
}

fn compose_mode_hint(
    registry_status: Option<RegistryStatus>,
    recovery_state: Option<RecoveryState>,
    offline_behavior: crate::domain::OfflineRegistryBehavior,
) -> ComposeMode {
    match registry_status {
        Some(RegistryStatus::Revoked) => ComposeMode::FailClosed,
        Some(RegistryStatus::Suspended) => ComposeMode::Restricted,
        Some(_) => match recovery_state {
            Some(RecoveryState::Broken)
            | Some(RecoveryState::Degraded)
            | Some(RecoveryState::Recovering) => ComposeMode::Degraded,
            Some(RecoveryState::Healthy) => ComposeMode::Normal,
            None => ComposeMode::BaselineOnly,
        },
        None => {
            if recovery_state.is_none() {
                ComposeMode::BaselineOnly
            } else {
                match offline_behavior {
                    crate::domain::OfflineRegistryBehavior::Cautious => ComposeMode::Degraded,
                    crate::domain::OfflineRegistryBehavior::BaselineOnly => {
                        ComposeMode::BaselineOnly
                    }
                    crate::domain::OfflineRegistryBehavior::FailClosed => ComposeMode::FailClosed,
                }
            }
        }
    }
}
