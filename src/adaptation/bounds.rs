pub fn clamp_trait_delta(delta: f32, max_trait_drift: f32) -> f32 {
    let max_trait_drift = max_trait_drift.max(0.0);
    delta.clamp(-max_trait_drift, max_trait_drift)
}

pub fn apply_trait_delta(baseline: f32, delta: f32, max_trait_drift: f32) -> f32 {
    (baseline + clamp_trait_delta(delta, max_trait_drift)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{apply_trait_delta, clamp_trait_delta};

    #[test]
    fn clamp_trait_delta_is_symmetric() {
        assert_eq!(clamp_trait_delta(0.25, 0.10), 0.10);
        assert_eq!(clamp_trait_delta(-0.25, 0.10), -0.10);
        assert_eq!(clamp_trait_delta(0.05, 0.10), 0.05);
    }

    #[test]
    fn apply_trait_delta_clamps_final_value_to_unit_interval() {
        assert_eq!(apply_trait_delta(0.95, 0.25, 0.10), 1.0);
        assert_eq!(apply_trait_delta(0.05, -0.25, 0.10), 0.0);
    }
}
