use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn stable_hash(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
