use std::cmp::Reverse;

use crate::domain::{
    BehaviorInputs, ComposeMode, ComposeRequest, NormalizedInputs, RecoveryState, RegistryStatus,
    RelationshipMarker, SoulError, WarningSeverity,
};

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

    let identity_snapshot = inputs.identity_snapshot.and_then(|mut snapshot| {
        if snapshot.agent_id != request.agent_id {
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
    let reputation_summary = if request.include_reputation && verification_result.is_some() {
        inputs.reputation_summary.map(|mut summary| {
            summary.context.sort();
            summary.context.dedup();
            summary
        })
    } else {
        None
    };

    let compose_mode_hint = compose_mode_hint(
        verification_result
            .as_ref()
            .map(|verification| verification.status),
        identity_snapshot
            .as_ref()
            .map(|snapshot| snapshot.recovery_state),
        inputs.soul_config.limits.offline_registry_behavior,
    );

    Ok(NormalizedInputs {
        schema_version: inputs.schema_version,
        request: request.clone(),
        agent_id: request.agent_id.clone(),
        profile_name: inputs.soul_config.profile_name.clone(),
        compose_mode_hint: Some(compose_mode_hint),
        identity_snapshot,
        verification_result,
        reputation_summary,
        soul_config: inputs.soul_config,
        adaptation_state: inputs.adaptation_state,
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
        None => match offline_behavior {
            crate::domain::OfflineRegistryBehavior::Cautious => ComposeMode::Degraded,
            crate::domain::OfflineRegistryBehavior::BaselineOnly => ComposeMode::BaselineOnly,
            crate::domain::OfflineRegistryBehavior::FailClosed => ComposeMode::FailClosed,
        },
    }
}
