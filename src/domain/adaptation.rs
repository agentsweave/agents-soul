#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdaptationState {
    pub notes: Vec<String>,
    pub bounded: bool,
}
