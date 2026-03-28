pub const DEFAULT_EMA_ALPHA: f32 = 0.10;
pub const LOSS_AVERSION_MULTIPLIER: f32 = 1.35;

pub fn ema_step(current: f32, signal: f32, alpha: f32) -> f32 {
    let alpha = alpha.clamp(0.0, 1.0);
    ((1.0 - alpha) * current) + (alpha * signal)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_EMA_ALPHA, ema_step};

    #[test]
    fn ema_step_biases_toward_history() {
        let next = ema_step(0.4, 1.0, DEFAULT_EMA_ALPHA);
        assert!((next - 0.46).abs() < f32::EPSILON);
    }

    #[test]
    fn ema_step_respects_alpha_bounds() {
        assert_eq!(ema_step(0.2, 0.8, 2.0), 0.8);
        assert_eq!(ema_step(0.2, 0.8, -1.0), 0.2);
    }
}
