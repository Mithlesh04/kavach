// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A seeded random number generator for generated tests (spec 11.9).

/// A small, fast random number generator (SplitMix64): the same seed
/// always gives the same values (spec 11.9).
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// A generator for a spawned task: its own numbers, the same each run.
    pub(crate) fn fork(&self, salt: u64) -> Self {
        let mut r = Rng(self.0 ^ salt.wrapping_mul(0xA24B_AED4_963E_E407));
        r.next();
        r
    }

    /// `seed` mixed with `text`: a seed of its own for each named thing.
    pub(crate) fn hash(seed: u64, text: &str) -> u64 {
        text.bytes().fold(seed, |h, b| {
            h.wrapping_mul(0x100_0000_01B3).wrapping_add(u64::from(b))
        })
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n` (`n` > 0).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// A number in `lo..=hi`.
    pub(crate) fn range(&mut self, lo: i128, hi: i128) -> i128 {
        let span = (hi - lo + 1) as u128;
        lo + ((self.next() as u128 * 0x1_0000_0001 + self.next() as u128) % span) as i128
    }
}
