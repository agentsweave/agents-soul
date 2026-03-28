#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelationshipsService;

impl RelationshipsService {
    pub const fn role() -> &'static str {
        "Attach relationship context that shapes downstream rendering."
    }
}
