//! Hashing for maps and sets: a fast mixing function with a seed chosen
//! at random when the program starts, so an attacker who controls the
//! keys cannot predict which keys collide. Split from `table.rs`.

use std::hash::{BuildHasher, Hasher};
use std::sync::OnceLock;

/// The random seed of this run.
pub(crate) fn seed() -> u64 {
    static SEED: OnceLock<u64> = OnceLock::new();
    *SEED.get_or_init(|| {
        std::collections::hash_map::RandomState::new().hash_one(0x6b61_7661_6368u64)
    })
}

/// Mixes `a` and `b` into one well-spread 64-bit value: the two halves of
/// their 128-bit product, combined.
pub(crate) fn mix(a: u64, b: u64) -> u64 {
    let m = (a as u128).wrapping_mul(b as u128);
    (m as u64) ^ ((m >> 64) as u64)
}

pub(crate) const K1: u64 = 0x9e37_79b9_7f4a_7c15;
pub(crate) const K2: u64 = 0xe703_7ed1_a0b4_28db;

/// A fast hasher for map keys, seeded per run.
pub struct FastHasher(u64);

impl FastHasher {
    pub fn new() -> Self {
        FastHasher(seed())
    }
}

impl Default for FastHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        mix(self.0, K2)
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = mix(self.0 ^ n, K1);
    }

    fn write(&mut self, bytes: &[u8]) {
        let (chunks, rest) = bytes.as_chunks::<8>();
        for c in chunks {
            self.write_u64(u64::from_le_bytes(*c));
        }
        let mut last = [0u8; 8];
        last[..rest.len()].copy_from_slice(rest);
        self.write_u64(u64::from_le_bytes(last) ^ ((bytes.len() as u64) << 56));
    }

    fn write_u8(&mut self, n: u8) {
        self.write_u64(n as u64);
    }

    fn write_u32(&mut self, n: u32) {
        self.write_u64(n as u64);
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }

    fn write_i64(&mut self, n: i64) {
        self.write_u64(n as u64);
    }
}
