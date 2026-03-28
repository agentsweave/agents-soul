#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionRulesService;

impl DecisionRulesService {
    pub const fn role() -> &'static str {
        "Render decision heuristics from normalized inputs."
    }
}
