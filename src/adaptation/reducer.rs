use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::domain::interactions::{
    AdaptiveTrait, InteractionOutcome, InteractionSignal, SignalDirection,
};
use crate::domain::{AdaptationState, HeuristicOverride, InteractionEvent, SoulConfig};

use super::{
    bounds::clamp_trait_delta,
    ema::{DEFAULT_EMA_ALPHA, LOSS_AVERSION_MULTIPLIER, ema_step},
    notes::{render_heuristic_note, render_trait_note},
};

const MIN_TRAIT_NOTE_DELTA: f32 = 0.005;
const MAX_HEURISTIC_PRIORITY_DELTA: i32 = 4;
const MIN_HEURISTIC_PRIORITY_SCORE: f32 = 0.35;
const TOGGLE_DECISION_MARGIN: f32 = 0.75;
const INSTRUCTION_DECISION_MARGIN: f32 = 0.75;

#[derive(Debug, Clone, PartialEq)]
pub struct InteractionReduction {
    pub adaptation_state: AdaptationState,
    pub interaction_count: u32,
    pub last_interaction_at: Option<DateTime<Utc>>,
}

pub fn reduce_interaction_evidence(
    config: &SoulConfig,
    interactions: &[InteractionEvent],
    reduced_at: DateTime<Utc>,
) -> InteractionReduction {
    let mut events = interactions
        .iter()
        .filter(|event| {
            event.agent_id == config.agent_id
                && event.recorded_at <= reduced_at
                && event.recorded_at >= window_start(config, reduced_at)
        })
        .cloned()
        .collect::<Vec<_>>();
    events.sort_by(|left, right| {
        (
            left.recorded_at,
            left.session_id.as_deref().unwrap_or_default(),
            left.interaction_type.as_str(),
            left.notes.as_deref().unwrap_or_default(),
        )
            .cmp(&(
                right.recorded_at,
                right.session_id.as_deref().unwrap_or_default(),
                right.interaction_type.as_str(),
                right.notes.as_deref().unwrap_or_default(),
            ))
    });

    let interaction_count = saturating_len(events.len());
    let last_interaction_at = events.last().map(|event| event.recorded_at);
    let mut adaptation_state = AdaptationState {
        schema_version: config.schema_version,
        last_updated_at: Some(reduced_at),
        evidence_window_size: interaction_count,
        ..AdaptationState::default()
    };

    if !config.adaptation.enabled
        || interaction_count < config.adaptation.min_interactions_for_adapt
    {
        return InteractionReduction {
            adaptation_state,
            interaction_count,
            last_interaction_at,
        };
    }

    let trait_accumulators = accumulate_trait_signals(&events);
    let heuristic_accumulators = accumulate_heuristic_signals(&events);

    for trait_name in [
        AdaptiveTrait::Openness,
        AdaptiveTrait::Conscientiousness,
        AdaptiveTrait::Initiative,
        AdaptiveTrait::Directness,
        AdaptiveTrait::Warmth,
        AdaptiveTrait::RiskTolerance,
        AdaptiveTrait::Verbosity,
        AdaptiveTrait::Formality,
    ] {
        let Some(accumulator) = trait_accumulators.get(&trait_name) else {
            continue;
        };
        let delta = clamp_trait_delta(
            accumulator.ema_score * config.limits.max_trait_drift,
            config.limits.max_trait_drift,
        );
        if delta.abs() < MIN_TRAIT_NOTE_DELTA {
            continue;
        }

        adaptation_state
            .trait_overrides
            .set_trait_delta(trait_name, delta);
        adaptation_state.notes.push(render_trait_note(
            trait_name.as_str(),
            delta,
            accumulator.signal_count,
            &dominant_reason(&accumulator.reasons),
        ));
    }

    let mut heuristic_overrides = heuristic_accumulators
        .into_iter()
        .filter_map(|(heuristic_id, accumulator)| {
            let priority_delta = priority_delta_from_score(accumulator.priority_score);
            let enabled =
                enabled_from_weights(accumulator.enable_weight, accumulator.disable_weight);
            let replacement_instruction = instruction_from_candidates(&accumulator.instructions);

            if priority_delta == 0 && enabled.is_none() && replacement_instruction.is_none() {
                return None;
            }

            let note = render_heuristic_note(
                &heuristic_id,
                priority_delta,
                enabled,
                replacement_instruction.is_some(),
                accumulator.signal_count,
                &dominant_reason(&accumulator.reasons),
            );

            Some(HeuristicOverride {
                heuristic_id,
                priority_delta,
                enabled,
                replacement_instruction,
                note: Some(note),
            })
        })
        .collect::<Vec<_>>();
    heuristic_overrides.sort_by(|left, right| left.heuristic_id.cmp(&right.heuristic_id));
    heuristic_overrides.truncate(config.limits.max_adaptive_rules);

    adaptation_state.notes.extend(
        heuristic_overrides
            .iter()
            .filter_map(|override_rule| override_rule.note.clone()),
    );
    adaptation_state.notes.sort();
    adaptation_state.notes.dedup();
    adaptation_state.heuristic_overrides = heuristic_overrides;

    InteractionReduction {
        adaptation_state,
        interaction_count,
        last_interaction_at,
    }
}

#[derive(Debug, Default)]
struct TraitAccumulator {
    ema_score: f32,
    signal_count: u32,
    reasons: BTreeMap<String, ReasonStats>,
}

#[derive(Debug, Default)]
struct HeuristicAccumulator {
    priority_score: f32,
    enable_weight: f32,
    disable_weight: f32,
    instructions: BTreeMap<String, InstructionStats>,
    signal_count: u32,
    reasons: BTreeMap<String, ReasonStats>,
}

#[derive(Debug, Default, Clone, Copy)]
struct ReasonStats {
    count: u32,
    total_weight: f32,
}

#[derive(Debug, Default, Clone, Copy)]
struct InstructionStats {
    count: u32,
    total_weight: f32,
}

fn accumulate_trait_signals(
    events: &[InteractionEvent],
) -> BTreeMap<AdaptiveTrait, TraitAccumulator> {
    let mut accumulators: BTreeMap<AdaptiveTrait, TraitAccumulator> = BTreeMap::new();

    for event in events {
        for signal in &event.signals {
            let InteractionSignal::Trait(signal) = signal else {
                continue;
            };

            let weight = weighted_signal(signal.strength, signal.direction, event.outcome);
            let accumulator = accumulators.entry(signal.trait_name).or_default();
            accumulator.ema_score = ema_step(accumulator.ema_score, weight, DEFAULT_EMA_ALPHA);
            accumulator.signal_count += 1;
            bump_reason(&mut accumulator.reasons, &signal.reason, weight.abs());
        }
    }

    accumulators
}

fn accumulate_heuristic_signals(
    events: &[InteractionEvent],
) -> BTreeMap<String, HeuristicAccumulator> {
    let mut accumulators: BTreeMap<String, HeuristicAccumulator> = BTreeMap::new();

    for event in events {
        for signal in &event.signals {
            match signal {
                InteractionSignal::HeuristicPriority(signal) => {
                    let weight = weighted_signal(signal.strength, signal.direction, event.outcome);
                    let accumulator = accumulators.entry(signal.heuristic_id.clone()).or_default();
                    accumulator.priority_score =
                        ema_step(accumulator.priority_score, weight, DEFAULT_EMA_ALPHA);
                    accumulator.signal_count += 1;
                    bump_reason(&mut accumulator.reasons, &signal.reason, weight.abs());
                }
                InteractionSignal::HeuristicToggle(signal) => {
                    let weight = outcome_weight(event.outcome);
                    let accumulator = accumulators.entry(signal.heuristic_id.clone()).or_default();
                    if signal.enabled {
                        accumulator.enable_weight += weight;
                    } else {
                        accumulator.disable_weight += weight;
                    }
                    accumulator.signal_count += 1;
                    bump_reason(&mut accumulator.reasons, &signal.reason, weight);
                }
                InteractionSignal::HeuristicInstruction(signal) => {
                    let weight = signal.strength.clamp(0.0, 1.0) * outcome_weight(event.outcome);
                    let accumulator = accumulators.entry(signal.heuristic_id.clone()).or_default();
                    let entry = accumulator
                        .instructions
                        .entry(signal.instruction.clone())
                        .or_default();
                    entry.count += 1;
                    entry.total_weight += weight;
                    accumulator.signal_count += 1;
                    bump_reason(&mut accumulator.reasons, &signal.reason, weight);
                }
                InteractionSignal::Trait(_) => {}
            }
        }
    }

    accumulators
}

fn weighted_signal(strength: f32, direction: SignalDirection, outcome: InteractionOutcome) -> f32 {
    let signed_strength = match direction {
        SignalDirection::Increase => strength.clamp(0.0, 1.0),
        SignalDirection::Decrease => -strength.clamp(0.0, 1.0),
    };
    signed_strength * outcome_weight(outcome)
}

fn outcome_weight(outcome: InteractionOutcome) -> f32 {
    match outcome {
        InteractionOutcome::Positive => 1.0,
        InteractionOutcome::Neutral => 0.85,
        InteractionOutcome::Negative => LOSS_AVERSION_MULTIPLIER,
    }
}

fn priority_delta_from_score(score: f32) -> i32 {
    if score.abs() < MIN_HEURISTIC_PRIORITY_SCORE {
        0
    } else {
        (score * MAX_HEURISTIC_PRIORITY_DELTA as f32).round().clamp(
            -MAX_HEURISTIC_PRIORITY_DELTA as f32,
            MAX_HEURISTIC_PRIORITY_DELTA as f32,
        ) as i32
    }
}

fn enabled_from_weights(enable_weight: f32, disable_weight: f32) -> Option<bool> {
    let difference = enable_weight - disable_weight;
    if difference.abs() < TOGGLE_DECISION_MARGIN {
        None
    } else {
        Some(difference.is_sign_positive())
    }
}

fn instruction_from_candidates(candidates: &BTreeMap<String, InstructionStats>) -> Option<String> {
    let mut best: Option<(&String, &InstructionStats)> = None;

    for candidate in candidates {
        match best {
            None => best = Some(candidate),
            Some((best_instruction, best_stats)) => {
                if candidate.1.total_weight > best_stats.total_weight
                    || (candidate.1.total_weight == best_stats.total_weight
                        && candidate.1.count > best_stats.count)
                    || (candidate.1.total_weight == best_stats.total_weight
                        && candidate.1.count == best_stats.count
                        && candidate.0 < best_instruction)
                {
                    best = Some(candidate);
                }
            }
        }
    }

    best.and_then(|(instruction, stats)| {
        (stats.total_weight >= INSTRUCTION_DECISION_MARGIN).then(|| instruction.clone())
    })
}

fn dominant_reason(reasons: &BTreeMap<String, ReasonStats>) -> String {
    reasons
        .iter()
        .max_by(|left, right| {
            left.1
                .total_weight
                .partial_cmp(&right.1.total_weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.1.count.cmp(&right.1.count))
                .then_with(|| right.0.cmp(left.0))
        })
        .map(|(reason, _)| reason.clone())
        .unwrap_or_else(|| "recent interaction evidence supported the override".to_owned())
}

fn bump_reason(reasons: &mut BTreeMap<String, ReasonStats>, reason: &str, weight: f32) {
    if reason.trim().is_empty() {
        return;
    }

    let stats = reasons.entry(reason.trim().to_owned()).or_default();
    stats.count += 1;
    stats.total_weight += weight;
}

fn window_start(config: &SoulConfig, reduced_at: DateTime<Utc>) -> DateTime<Utc> {
    reduced_at - Duration::days(i64::from(config.adaptation.learning_window_days))
}

fn saturating_len(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use crate::{
        adaptation::{
            materialize_effective_overrides, store::AdaptiveWriteRequest,
            store::persist_adaptation_write,
        },
        domain::interactions::{
            AdaptiveTrait, HeuristicInstructionSignal, HeuristicPrioritySignal,
            HeuristicToggleSignal, InteractionOutcome, InteractionSignal, SignalDirection,
            TraitSignal,
        },
        domain::{DecisionHeuristic, PersonalityOverride, SoulConfig},
        storage::sqlite::open_database,
    };

    use super::reduce_interaction_evidence;

    #[test]
    fn reducer_filters_old_evidence_and_clamps_trait_drift() {
        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.limits.max_trait_drift = 0.10;
        config.adaptation.learning_window_days = 7;
        config.adaptation.min_interactions_for_adapt = 3;

        let reduction = reduce_interaction_evidence(
            &config,
            &[
                event(
                    "agent.alpha",
                    InteractionOutcome::Negative,
                    ts(2026, 3, 28, 1, 0, 0),
                    vec![InteractionSignal::Trait(TraitSignal {
                        trait_name: AdaptiveTrait::Verbosity,
                        direction: SignalDirection::Decrease,
                        strength: 1.0,
                        reason: "user preferred concise responses".to_owned(),
                    })],
                ),
                event(
                    "agent.alpha",
                    InteractionOutcome::Negative,
                    ts(2026, 3, 28, 2, 0, 0),
                    vec![InteractionSignal::Trait(TraitSignal {
                        trait_name: AdaptiveTrait::Verbosity,
                        direction: SignalDirection::Decrease,
                        strength: 0.9,
                        reason: "user preferred concise responses".to_owned(),
                    })],
                ),
                event(
                    "agent.alpha",
                    InteractionOutcome::Positive,
                    ts(2026, 3, 28, 3, 0, 0),
                    vec![InteractionSignal::Trait(TraitSignal {
                        trait_name: AdaptiveTrait::Verbosity,
                        direction: SignalDirection::Increase,
                        strength: 0.2,
                        reason: "more detail helped once".to_owned(),
                    })],
                ),
                event(
                    "agent.alpha",
                    InteractionOutcome::Negative,
                    ts(2026, 3, 10, 3, 0, 0),
                    vec![InteractionSignal::Trait(TraitSignal {
                        trait_name: AdaptiveTrait::Verbosity,
                        direction: SignalDirection::Increase,
                        strength: 1.0,
                        reason: "too old to count".to_owned(),
                    })],
                ),
            ],
            ts(2026, 3, 29, 3, 0, 0),
        );

        assert_eq!(reduction.interaction_count, 3);
        assert!(reduction.adaptation_state.trait_overrides.verbosity < 0.0);
        assert!(reduction.adaptation_state.trait_overrides.verbosity >= -0.10);
        assert_eq!(
            reduction.adaptation_state.notes,
            vec![
                "verbosity reduced by 0.02 from 3 recent signals; user preferred concise responses"
            ]
        );
    }

    #[test]
    fn reducer_derives_heuristic_overrides_and_persists_cleanly()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.adaptation.min_interactions_for_adapt = 3;
        config.decision_heuristics = vec![DecisionHeuristic {
            heuristic_id: "review-risk".to_owned(),
            title: "Review Risk".to_owned(),
            priority: 2,
            trigger: "review".to_owned(),
            instruction: "Use the baseline review heuristic.".to_owned(),
            enabled: true,
            ..DecisionHeuristic::default()
        }];

        let reduced_at = ts(2026, 3, 29, 4, 0, 0);
        let reduction = reduce_interaction_evidence(
            &config,
            &[
                event(
                    "agent.alpha",
                    InteractionOutcome::Negative,
                    ts(2026, 3, 29, 1, 0, 0),
                    vec![
                        InteractionSignal::HeuristicPriority(HeuristicPrioritySignal {
                            heuristic_id: "review-risk".to_owned(),
                            direction: SignalDirection::Increase,
                            strength: 1.0,
                            reason: "missed risky change in review".to_owned(),
                        }),
                        InteractionSignal::HeuristicToggle(HeuristicToggleSignal {
                            heuristic_id: "review-risk".to_owned(),
                            enabled: true,
                            reason: "review safeguard is still needed".to_owned(),
                        }),
                    ],
                ),
                event(
                    "agent.alpha",
                    InteractionOutcome::Negative,
                    ts(2026, 3, 29, 2, 0, 0),
                    vec![InteractionSignal::HeuristicInstruction(
                        HeuristicInstructionSignal {
                            heuristic_id: "review-risk".to_owned(),
                            instruction: "Escalate risky diffs and verify test coverage."
                                .to_owned(),
                            strength: 1.0,
                            reason: "missed risky change in review".to_owned(),
                        },
                    )],
                ),
                event(
                    "agent.alpha",
                    InteractionOutcome::Negative,
                    ts(2026, 3, 29, 3, 0, 0),
                    vec![InteractionSignal::HeuristicPriority(
                        HeuristicPrioritySignal {
                            heuristic_id: "review-risk".to_owned(),
                            direction: SignalDirection::Increase,
                            strength: 0.9,
                            reason: "missed risky change in review".to_owned(),
                        },
                    )],
                ),
            ],
            reduced_at,
        );

        let request =
            AdaptiveWriteRequest::from_reduction("agent.alpha", true, reduced_at, &reduction);
        let db_path = temp_path("interaction-reducer");
        let conn = open_database(&db_path)?;
        let write = persist_adaptation_write(&conn, &config, &request)?;
        let stored = write.stored_state.expect("stored state");
        let effective = materialize_effective_overrides(&config, Some(&stored));

        assert_eq!(reduction.interaction_count, 3);
        assert_eq!(stored.interaction_count, 3);
        assert_eq!(effective.adaptation_state.heuristic_overrides.len(), 1);
        assert_eq!(
            effective.adaptation_state.heuristic_overrides[0]
                .replacement_instruction
                .as_deref(),
            Some("Escalate risky diffs and verify test coverage.")
        );
        assert!(
            effective
                .adaptation_state
                .notes
                .iter()
                .any(|note| note.contains("Heuristic `review-risk`")
                    && note.contains("missed risky change in review"))
        );
        if db_path.exists() {
            fs::remove_file(&db_path)?;
        }

        Ok(())
    }

    #[test]
    fn reducer_stays_empty_until_minimum_interaction_count_is_met() {
        let mut config = SoulConfig {
            agent_id: "agent.alpha".to_owned(),
            profile_name: "Alpha".to_owned(),
            ..SoulConfig::default()
        };
        config.adaptation.min_interactions_for_adapt = 4;

        let reduction = reduce_interaction_evidence(
            &config,
            &[
                event(
                    "agent.alpha",
                    InteractionOutcome::Positive,
                    ts(2026, 3, 29, 1, 0, 0),
                    vec![InteractionSignal::Trait(TraitSignal {
                        trait_name: AdaptiveTrait::Warmth,
                        direction: SignalDirection::Increase,
                        strength: 1.0,
                        reason: "collaboration went smoothly".to_owned(),
                    })],
                ),
                event(
                    "agent.alpha",
                    InteractionOutcome::Positive,
                    ts(2026, 3, 29, 2, 0, 0),
                    vec![InteractionSignal::Trait(TraitSignal {
                        trait_name: AdaptiveTrait::Warmth,
                        direction: SignalDirection::Increase,
                        strength: 1.0,
                        reason: "collaboration went smoothly".to_owned(),
                    })],
                ),
                event(
                    "agent.alpha",
                    InteractionOutcome::Positive,
                    ts(2026, 3, 29, 3, 0, 0),
                    vec![InteractionSignal::Trait(TraitSignal {
                        trait_name: AdaptiveTrait::Warmth,
                        direction: SignalDirection::Increase,
                        strength: 1.0,
                        reason: "collaboration went smoothly".to_owned(),
                    })],
                ),
            ],
            ts(2026, 3, 29, 4, 0, 0),
        );

        assert_eq!(reduction.interaction_count, 3);
        assert_eq!(
            reduction.adaptation_state.trait_overrides,
            PersonalityOverride::default()
        );
        assert!(reduction.adaptation_state.heuristic_overrides.is_empty());
        assert!(reduction.adaptation_state.notes.is_empty());
    }

    fn event(
        agent_id: &str,
        outcome: InteractionOutcome,
        recorded_at: chrono::DateTime<Utc>,
        signals: Vec<InteractionSignal>,
    ) -> crate::domain::InteractionEvent {
        crate::domain::InteractionEvent {
            agent_id: agent_id.to_owned(),
            session_id: Some("session.alpha".to_owned()),
            interaction_type: "review".to_owned(),
            outcome,
            signals,
            notes: None,
            recorded_at,
        }
    }

    fn ts(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .expect("valid timestamp")
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("agents-soul-{label}-{suffix}.sqlite"))
    }
}
