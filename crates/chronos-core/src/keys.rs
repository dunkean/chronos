//! Stable decision keys.
//!
//! Every random decision is a pure function of the seed and a semantic key
//! (`"war"`, year, polity IDs, ...). There is no shared RNG state, so the
//! order in which decisions are evaluated never changes their outcome.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a over bytes. Used for fingerprints and as the key mixer.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Hex fingerprint of arbitrary bytes (64-bit, not cryptographic).
pub fn fingerprint(bytes: &[u8]) -> String {
    format!("fnv1a64:{:016x}", fnv1a(bytes))
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Hash of the seed and an ordered list of key parts.
pub fn key(seed: u64, parts: &[&str]) -> u64 {
    let mut h = FNV_OFFSET ^ splitmix(seed);
    for part in parts {
        for b in part.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(FNV_PRIME);
        }
        // Separator so that ["ab","c"] and ["a","bc"] differ.
        h ^= 0xff;
        h = h.wrapping_mul(FNV_PRIME);
    }
    splitmix(h)
}

/// Uniform value in `[0, 1)` for a semantic key.
pub fn unit(seed: u64, parts: &[&str]) -> f64 {
    (key(seed, parts) >> 11) as f64 / (1u64 << 53) as f64
}

/// Uniform integer in `lo..=hi` for a semantic key.
pub fn range_i(seed: u64, parts: &[&str], lo: i64, hi: i64) -> i64 {
    debug_assert!(lo <= hi);
    lo + (key(seed, parts) % ((hi - lo + 1) as u64)) as i64
}
