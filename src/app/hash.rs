use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};

use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn stable_hash(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub fn stable_content_hash(content: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(content.as_ref());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        // Writing into a String cannot fail; this is infallible in practice.
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

pub fn stable_json_hash<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let payload = serde_json::to_vec(value)?;
    Ok(stable_content_hash(payload))
}

pub fn sorted_dedup_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort_unstable();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::{sorted_dedup_strings, stable_content_hash, stable_json_hash};

    #[derive(Debug, Serialize)]
    struct Sample<'a> {
        name: &'a str,
        score: u32,
    }

    #[test]
    fn stable_content_hash_is_repeatable() {
        let first = stable_content_hash("agents-soul");
        let second = stable_content_hash("agents-soul");

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn stable_json_hash_is_repeatable() {
        let payload = Sample {
            name: "alpha",
            score: 7,
        };

        let first = stable_json_hash(&payload).expect("json hashing should succeed");
        let second = stable_json_hash(&payload).expect("json hashing should succeed");

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn sorted_dedup_strings_returns_stable_order() {
        let ordered = sorted_dedup_strings(vec![
            "zeta".to_owned(),
            "alpha".to_owned(),
            "zeta".to_owned(),
            "beta".to_owned(),
        ]);

        assert_eq!(ordered, vec!["alpha", "beta", "zeta"]);
    }
}
