#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityReader;

impl IdentityReader {
    pub const fn role() -> &'static str {
        "Read upstream identity snapshots without shaping behavior."
    }
}
