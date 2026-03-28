#[derive(Debug, Clone, PartialEq)]
pub struct PersonalityProfile {
    pub openness: f32,
    pub conscientiousness: f32,
    pub initiative: f32,
    pub directness: f32,
    pub warmth: f32,
    pub risk_tolerance: f32,
    pub verbosity: f32,
    pub formality: f32,
}

impl Default for PersonalityProfile {
    fn default() -> Self {
        Self {
            openness: 0.72,
            conscientiousness: 0.90,
            initiative: 0.84,
            directness: 0.81,
            warmth: 0.42,
            risk_tolerance: 0.28,
            verbosity: 0.34,
            formality: 0.71,
        }
    }
}
