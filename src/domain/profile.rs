use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonalityProfile {
    pub name: String,
    pub traits: BTreeMap<String, u8>,
}
