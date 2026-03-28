use serde::{Deserialize, Serialize};

use super::SoulError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl PersonalityProfile {
    pub fn validate(&self) -> Result<(), SoulError> {
        validate_unit_interval("openness", self.openness)?;
        validate_unit_interval("conscientiousness", self.conscientiousness)?;
        validate_unit_interval("initiative", self.initiative)?;
        validate_unit_interval("directness", self.directness)?;
        validate_unit_interval("warmth", self.warmth)?;
        validate_unit_interval("risk_tolerance", self.risk_tolerance)?;
        validate_unit_interval("verbosity", self.verbosity)?;
        validate_unit_interval("formality", self.formality)?;
        Ok(())
    }
}

fn validate_unit_interval(field: &'static str, value: f32) -> Result<(), SoulError> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(SoulError::InvalidTraitValue { field, value })
    }
}
