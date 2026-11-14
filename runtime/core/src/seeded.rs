//! The numbers of `random.seeded(seed) { … }` (spec 14.15, decision
//! 0107): xoshiro256**, its state made from the seed by SplitMix64, as
//! their published versions (Blackman and Vigna) make them. Named in the
//! spec, so a seed gives the same numbers in every version of Kavach.

/// SplitMix64's next output, moving `state` on.
pub fn split_mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A xoshiro256** generator.
#[derive(Clone, Debug)]
pub struct Seeded {
    s: [u64; 4],
}

impl Seeded {
    /// The generator for `seed`.
    pub fn new(seed: u64) -> Seeded {
        let mut state = seed;
        Seeded {
            s: std::array::from_fn(|_| split_mix(&mut state)),
        }
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_mix_matches_its_published_outputs() {
        // The reference implementation's first outputs from seed 0.
        let mut state = 0;
        assert_eq!(split_mix(&mut state), 0xe220_a839_7b1d_cdaf);
        assert_eq!(split_mix(&mut state), 0x6e78_9e6a_a1b9_65f4);
    }

    #[test]
    fn a_seed_gives_the_same_numbers_and_seeds_differ() {
        let (mut a, mut b, mut c) = (Seeded::new(7), Seeded::new(7), Seeded::new(8));
        let first: Vec<u64> = (0..5).map(|_| a.next_u64()).collect();
        assert_eq!(first, (0..5).map(|_| b.next_u64()).collect::<Vec<_>>());
        assert_ne!(first[0], c.next_u64());
    }
}
