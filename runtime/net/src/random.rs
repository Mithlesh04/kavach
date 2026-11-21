// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Random bits for `std.random` (spec 14.15): from the operating
//! system's cryptographically secure generator, through the `ring`
//! provider TLS already uses (decision 0021), so no new dependency. In
//! this crate so a program without random numbers or networking doesn't
//! link it (decision 0024).

/// 64 random bits.
pub fn bits() -> u64 {
    let mut b = [0u8; 8];
    // The system generator fails only when the system has no randomness
    // to give, and nothing random can be made then.
    rustls::crypto::ring::default_provider()
        .secure_random
        .fill(&mut b)
        .expect("the operating system gave no random bytes");
    u64::from_le_bytes(b)
}

/// 64 random bits at a time, from a buffer the system generator fills
/// 512 at a time, for drawing many numbers at once.
pub fn many_bits() -> impl FnMut() -> u64 {
    let mut buffer = [0u64; 512];
    let mut next = buffer.len();
    move || {
        if next == buffer.len() {
            let mut b = [0u8; 4096];
            rustls::crypto::ring::default_provider()
                .secure_random
                .fill(&mut b)
                .expect("the operating system gave no random bytes");
            for (slot, word) in buffer.iter_mut().zip(b.as_chunks::<8>().0) {
                *slot = u64::from_le_bytes(*word);
            }
            next = 0;
        }
        next += 1;
        buffer[next - 1]
    }
}

/// `n` whole numbers from `from` up to, not including, `until`, from
/// `bits`: exactly what `n` calls of `std.random`'s `int` give from the
/// same bits (`stdlib/random/random.kav`), so a `random.seeded` block
/// gives the same numbers either way (spec 14.15). None when the range
/// is empty: the caller's `require` rules that out.
pub fn ints(n: usize, from: i64, until: i64, mut bits: impl FnMut() -> u64) -> Vec<i64> {
    if until <= from {
        return Vec::new();
    }
    let mut bits63 = move || (bits() & i64::MAX as u64) as i64;
    let mut out = Vec::with_capacity(n);
    match until.checked_sub(from) {
        // A range wider than any `Int`: any 64 bits, kept when in it.
        None => {
            while out.len() < n {
                let high = bits63().wrapping_mul(2);
                let v = high.wrapping_add(bits63() % 2);
                if v >= from && v < until {
                    out.push(v);
                }
            }
        }
        Some(span) => {
            // The largest multiple of `span` of the 2^63 values, so no
            // number comes up more often than another.
            let limit = i64::MAX - (i64::MAX % span + 1) % span;
            for _ in 0..n {
                let mut r = bits63();
                while r > limit {
                    r = bits63();
                }
                out.push(from + r % span);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{bits, ints, many_bits};

    #[test]
    fn draws_differ() {
        // 64 equal bits twice in a row would happen once in 2^64 runs.
        assert_ne!(bits(), bits());
        let mut many = many_bits();
        let drawn: Vec<u64> = (0..1000).map(|_| many()).collect();
        assert_ne!(drawn[0], drawn[999]);
    }

    /// Bits counting up from `start`, for draws that can be checked.
    fn counting(start: u64) -> impl FnMut() -> u64 {
        let mut next = start;
        move || {
            next = next.wrapping_add(1);
            next.wrapping_sub(1)
        }
    }

    #[test]
    fn ints_stay_in_range() {
        let cases: &[(i64, i64)] = &[
            (0, 10),
            (-5, 5),
            (7, 8),
            (i64::MIN, i64::MAX),
            (i64::MIN, 0),
            (-1, i64::MAX),
        ];
        for &(from, until) in cases {
            // Mixed bits (SplitMix64), so a range of any part is reached.
            let mut state = 7u64;
            let mixed = move || {
                state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let z = (state ^ (state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                z ^ (z >> 31)
            };
            let got = ints(300, from, until, mixed);
            assert_eq!(got.len(), 300);
            assert!(
                got.iter().all(|v| *v >= from && *v < until),
                "{from}..{until}"
            );
        }
    }

    #[test]
    fn ints_take_bits_as_random_int_does() {
        // 63 bits each, then the remainder: 5 % 3, 6 % 3, 7 % 3.
        assert_eq!(ints(3, 10, 13, counting(5)), vec![12, 10, 11]);
        // The top bit is dropped, as `random.int`'s `bits63` drops it.
        assert_eq!(ints(1, 0, 10, counting(1 << 63 | 4)), vec![4]);
        // A draw above the largest multiple of the span is drawn again:
        // 2^63 - 1 leaves 1 over 3, so the two largest are.
        assert_eq!(ints(1, 0, 3, counting(i64::MAX as u64 - 1)), vec![0]);
        // A range wider than `Int`: two draws make one number, 3 * 2 + 4 % 2.
        assert_eq!(ints(1, i64::MIN, i64::MAX, counting(3)), vec![6]);
    }
}
