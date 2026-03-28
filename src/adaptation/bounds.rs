#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdaptationBounds;

impl AdaptationBounds {
    pub const fn role() -> &'static str {
        "Clamp adaptive drift before it reaches composition."
    }
}
