pub fn stable_hash(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
}
