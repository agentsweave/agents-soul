#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegistryReader;

impl RegistryReader {
    pub const fn role() -> &'static str {
        "Read registry standing and reputation without becoming registry authority."
    }
}
