#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SqliteStorage;

impl SqliteStorage {
    pub const fn role() -> &'static str {
        "Own durable local persistence for adaptation and fixtures."
    }
}
