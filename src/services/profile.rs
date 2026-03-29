use crate::domain::{ComposeMode, NormalizedInputs, PersonalityProfile};

#[derive(Debug, Clone, Default)]
pub struct EffectiveProfileService;

impl EffectiveProfileService {
    pub fn derive_baseline(&self, normalized: &NormalizedInputs) -> PersonalityProfile {
        normalized.soul_config.trait_baseline.clone()
    }

    pub fn derive(
        &self,
        normalized: &NormalizedInputs,
        compose_mode: ComposeMode,
    ) -> PersonalityProfile {
        let mut profile = self.derive_baseline(normalized);

        if normalized.soul_config.adaptation.enabled {
            let overrides = &normalized.adaptation_state.trait_overrides;
            let max_drift = normalized.soul_config.limits.max_trait_drift;

            profile.openness = apply_drift(profile.openness, overrides.openness, max_drift);
            profile.conscientiousness = apply_drift(
                profile.conscientiousness,
                overrides.conscientiousness,
                max_drift,
            );
            profile.initiative = apply_drift(profile.initiative, overrides.initiative, max_drift);
            profile.directness = apply_drift(profile.directness, overrides.directness, max_drift);
            profile.warmth = apply_drift(profile.warmth, overrides.warmth, max_drift);
            profile.risk_tolerance =
                apply_drift(profile.risk_tolerance, overrides.risk_tolerance, max_drift);
            profile.verbosity = apply_drift(profile.verbosity, overrides.verbosity, max_drift);
            profile.formality = apply_drift(profile.formality, overrides.formality, max_drift);
        }

        apply_mode_bounds(&mut profile, compose_mode);
        profile
    }
}

fn apply_drift(base: f32, delta: f32, max_drift: f32) -> f32 {
    clamp_unit(base + delta.clamp(-max_drift, max_drift))
}

fn clamp_unit(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn cap(value: &mut f32, ceiling: f32) {
    *value = value.min(ceiling);
}

fn floor(value: &mut f32, minimum: f32) {
    *value = value.max(minimum);
}

fn apply_mode_bounds(profile: &mut PersonalityProfile, compose_mode: ComposeMode) {
    match compose_mode {
        ComposeMode::Normal | ComposeMode::BaselineOnly => {}
        ComposeMode::Degraded => {
            cap(&mut profile.initiative, 0.55);
            cap(&mut profile.risk_tolerance, 0.18);
            floor(&mut profile.formality, 0.72);
            floor(&mut profile.conscientiousness, 0.88);
        }
        ComposeMode::Restricted => {
            cap(&mut profile.initiative, 0.35);
            cap(&mut profile.risk_tolerance, 0.12);
            cap(&mut profile.directness, 0.70);
            floor(&mut profile.formality, 0.75);
            floor(&mut profile.conscientiousness, 0.90);
        }
        ComposeMode::FailClosed => {
            cap(&mut profile.initiative, 0.05);
            cap(&mut profile.risk_tolerance, 0.02);
            cap(&mut profile.directness, 0.40);
            cap(&mut profile.verbosity, 0.25);
            floor(&mut profile.formality, 0.82);
            floor(&mut profile.conscientiousness, 0.95);
            floor(&mut profile.warmth, 0.45);
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::domain::{
        AdaptationState, BehaviorInputs, ComposeMode, ComposeRequest, PersonalityOverride,
        SoulConfig,
    };
    use crate::sources::normalize::normalize_inputs;

    use super::EffectiveProfileService;

    #[test]
    fn derive_clamps_adaptive_trait_drift() {
        let request = ComposeRequest::new("alpha", "session-1");
        let mut config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        config.limits.max_trait_drift = 0.10;

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                adaptation_state: AdaptationState {
                    trait_overrides: PersonalityOverride {
                        risk_tolerance: 0.45,
                        initiative: -0.30,
                        ..PersonalityOverride::default()
                    },
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let service = EffectiveProfileService;
        let profile = service.derive(&normalized, ComposeMode::Normal);

        assert!((profile.risk_tolerance - 0.38).abs() < f32::EPSILON);
        assert!((profile.initiative - 0.74).abs() < f32::EPSILON);
    }

    #[test]
    fn restricted_mode_visibly_reduces_autonomy() {
        let request = ComposeRequest::new("alpha", "session-1");
        let config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let service = EffectiveProfileService;
        let profile = service.derive(&normalized, ComposeMode::Restricted);

        assert!(profile.initiative <= 0.35);
        assert!(profile.risk_tolerance <= 0.12);
        assert!(profile.formality >= 0.75);
    }

    #[test]
    fn baseline_profile_stays_inspectable_alongside_effective_profile() {
        let request = ComposeRequest::new("alpha", "session-1");
        let mut config = SoulConfig {
            agent_id: "alpha".into(),
            profile_name: "Alpha".into(),
            ..SoulConfig::default()
        };
        config.limits.max_trait_drift = 0.10;

        let normalized = normalize_inputs(
            &request,
            BehaviorInputs {
                soul_config: config,
                adaptation_state: AdaptationState {
                    trait_overrides: PersonalityOverride {
                        directness: 0.25,
                        risk_tolerance: 0.50,
                        ..PersonalityOverride::default()
                    },
                    ..AdaptationState::default()
                },
                generated_at: Utc::now(),
                ..BehaviorInputs::default()
            },
        )
        .expect("normalized inputs");

        let service = EffectiveProfileService;
        let baseline = service.derive_baseline(&normalized);
        let effective = service.derive(&normalized, ComposeMode::Degraded);

        assert_eq!(
            baseline.directness,
            normalized.soul_config.trait_baseline.directness
        );
        assert_eq!(
            baseline.risk_tolerance,
            normalized.soul_config.trait_baseline.risk_tolerance
        );
        assert!((effective.directness - 0.91).abs() < f32::EPSILON);
        assert!(effective.risk_tolerance <= 0.18);
    }
}
