#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileService;

impl ProfileService {
    pub const fn role() -> &'static str {
        "Layer baseline profile data before adaptation and rendering."
    }
}
