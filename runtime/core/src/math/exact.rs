// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Exact sums of floats, rounded once (spec 14.2, decision 0135): a
//! list's `sum(f)` of `Float`s or `Float32`s. Every finite `Float` is a
//! whole number of 2^-1074, so each is added into a wide fixed-point
//! number, kept as 32-bit pieces in `i64`s with room for carries. No
//! order of adding changes the result, so it has the same bits on one
//! core, on all cores and on the GPU. The interpreter and native code
//! (`kv_rt_exact`) both use this.
//!
//! A long sum first adds each value's 53 bits into a bin for its
//! exponent, one add of whole numbers, and moves a bin into the pieces
//! only when it nears 2^62, and at the end: a few times the cost of a
//! plain float add.

/// Pieces of 32 bits: 2^-1074 up to past 2^1100, with room for 2^63
/// values and the sign.
const LIMBS: usize = 70;

/// How many values may be put into the pieces before the carries are
/// passed up: each adds less than 2^32 to a piece, and a piece holds 2^62.
const CARRY_EVERY: u32 = 1 << 30;

/// How many values go straight into the pieces before a sum gets bins:
/// a short sum, such as each key's in a sum per key (14.2), stays small.
const BINS_AFTER: u32 = 64;

/// One bin for each exponent: the sum of the bits of the values that
/// have it.
type Bins = [i64; 2047];

/// A sum of floats being worked out. Only the pieces its values reach
/// are kept, so a short sum of numbers of like size is small.
#[derive(Clone, Debug)]
pub struct ExactSum {
    /// Pieces `base` up: piece `k` counts `2^(32k − 1074)`s.
    limbs: Vec<i64>,
    base: usize,
    bins: Option<Box<Bins>>,
    nan: bool,
    up: bool,
    down: bool,
    /// Every value so far was `-0.0` (or there was none).
    negative_zeros: bool,
    /// Some value was added.
    seen: bool,
    /// Values put into the pieces so far.
    placed: u32,
}

impl Default for ExactSum {
    fn default() -> Self {
        ExactSum {
            limbs: Vec::new(),
            base: 0,
            bins: None,
            nan: false,
            up: false,
            down: false,
            negative_zeros: true,
            seen: false,
            placed: 0,
        }
    }
}

impl ExactSum {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `x`, exactly. A `Float32` is added as the `Float` it widens
    /// to, which is the same number.
    #[inline]
    pub fn add(&mut self, x: f64) {
        let b = x.to_bits();
        let e = ((b >> 52) & 0x7ff) as usize;
        let m = b & ((1 << 52) - 1);
        let negative = b >> 63 != 0;
        self.seen = true;
        // The value is `m × 2^(p − 1074)`: `m` at bit `p` of the sum.
        let (m, p) = match e {
            0x7ff => return self.special(m != 0, negative),
            0 if m == 0 => {
                self.negative_zeros &= negative;
                return;
            }
            0 => (m, 0),
            _ => (m | 1 << 52, e - 1),
        };
        self.negative_zeros = false;
        if let Some(bins) = &mut self.bins {
            let bin = &mut bins[p];
            *bin += if negative { -(m as i64) } else { m as i64 };
            if bin.unsigned_abs() >= 1 << 62 {
                let v = std::mem::take(bin);
                self.place(p, v.unsigned_abs(), v < 0);
            }
            return;
        }
        self.place(p, m, negative);
        if self.placed == BINS_AFTER {
            self.bins = Some(Box::new([0; 2047]));
        }
    }

    /// A NaN or an infinity.
    #[cold]
    fn special(&mut self, nan: bool, negative: bool) {
        match (nan, negative) {
            (true, _) => self.nan = true,
            (false, true) => self.down = true,
            (false, false) => self.up = true,
        }
    }

    /// Adds `m × 2^(p − 1074)`, negated when `negative`, to the pieces;
    /// `m` is below 2^63.
    fn place(&mut self, p: usize, m: u64, negative: bool) {
        let shifted = u128::from(m) << (p % 32);
        let i = self.reach(p / 32, 3);
        let parts = [
            (shifted & 0xffff_ffff) as i64,
            ((shifted >> 32) & 0xffff_ffff) as i64,
            (shifted >> 64) as i64,
        ];
        for (k, v) in parts.into_iter().enumerate() {
            if negative {
                self.limbs[i + k] -= v;
            } else {
                self.limbs[i + k] += v;
            }
        }
        self.placed = self.placed.wrapping_add(1);
        if self.placed.is_multiple_of(CARRY_EVERY) {
            self.carry();
        }
    }

    /// Makes room for pieces `at` to `at + n - 1`; gives where `at` is
    /// kept.
    fn reach(&mut self, at: usize, n: usize) -> usize {
        if self.limbs.is_empty() {
            self.base = at;
            self.limbs.resize(n, 0);
        }
        if at < self.base {
            let more = self.base - at;
            self.limbs.splice(0..0, std::iter::repeat_n(0, more));
            self.base = at;
        }
        let end = at + n - self.base;
        if end > self.limbs.len() {
            self.limbs.resize(end, 0);
        }
        at - self.base
    }

    /// The same sum with its bins emptied into its pieces.
    fn emptied(&self) -> ExactSum {
        let mut s = self.clone();
        if let Some(bins) = s.bins.take() {
            for (p, v) in bins.iter().enumerate().filter(|(_, v)| **v != 0) {
                s.place(p, v.unsigned_abs(), *v < 0);
            }
        }
        s
    }

    /// Adds every value another sum holds, as if each were added here.
    pub fn merge(&mut self, other: &ExactSum) {
        let other = other.emptied();
        if !other.limbs.is_empty() {
            let i = self.reach(other.base, other.limbs.len());
            for (k, v) in other.limbs.iter().enumerate() {
                self.limbs[i + k] += v;
            }
            self.carry();
        }
        self.nan |= other.nan;
        self.up |= other.up;
        self.down |= other.down;
        self.negative_zeros &= other.negative_zeros;
        self.seen |= other.seen;
    }

    /// Passes every carry up: each piece from 0 up to 2^32 but the last,
    /// which keeps the sign, with more pieces as the sum grows.
    fn carry(&mut self) {
        for i in 0..self.limbs.len().saturating_sub(1) {
            let c = self.limbs[i] >> 32;
            self.limbs[i] -= c << 32;
            self.limbs[i + 1] += c;
        }
        while let Some(&top) = self.limbs.last()
            && !(-1..=0).contains(&(top >> 32))
        {
            let c = top >> 32;
            *self.limbs.last_mut().expect("not empty") -= c << 32;
            self.limbs.push(c);
        }
    }

    /// The sum rounded once to a `Float`.
    pub fn value(&self) -> f64 {
        self.rounded(53, 0)
    }

    /// The sum rounded once to a `Float32`, with no rounding to a
    /// `Float` on the way.
    pub fn value32(&self) -> f32 {
        // Rounded to 24 bits and the `Float32` grid already, so the
        // `Float` is exactly a `Float32`, or past the largest one.
        self.rounded(24, 1074 - 149) as f32
    }

    /// The sum rounded to the nearest number of `precision` bits whose
    /// lowest bit is at least bit `lowest` of the sum (where the type's
    /// smallest number is), ties to even.
    fn rounded(&self, precision: i64, lowest: i64) -> f64 {
        if self.nan || (self.up && self.down) {
            return f64::NAN;
        }
        if self.up {
            return f64::INFINITY;
        }
        if self.down {
            return f64::NEG_INFINITY;
        }
        // Every piece, from the lowest.
        let all = self.emptied();
        let mut limbs = vec![0; LIMBS];
        for (k, v) in all.limbs.iter().enumerate() {
            limbs[all.base + k] += v;
        }
        let mut s = ExactSum {
            limbs,
            ..ExactSum::default()
        };
        s.carry();
        let negative = s.limbs[LIMBS - 1] < 0;
        if negative {
            for v in s.limbs.iter_mut() {
                *v = -*v;
            }
            s.carry();
        }
        let bit = |n: i64| n >= 0 && (s.limbs[(n / 32) as usize] >> (n % 32)) & 1 == 1;
        let Some(top) = (0..LIMBS).rev().find(|&i| s.limbs[i] != 0) else {
            return if all.seen && all.negative_zeros {
                -0.0
            } else {
                0.0
            };
        };
        let high = top as i64 * 32 + 63 - i64::from(s.limbs[top].leading_zeros());
        let mut low = (high - (precision - 1)).max(lowest);
        let mut q: u64 = 0;
        for n in (low..=high).rev() {
            q = q << 1 | u64::from(bit(n));
        }
        let half = bit(low - 1);
        let below = (0..low - 1).any(bit);
        if half && (below || q & 1 == 1) {
            q += 1;
            if q == 1 << precision {
                q >>= 1;
                low += 1;
            }
        }
        let magnitude = libm::scalbn(q as f64, (low - 1074) as i32);
        if negative { -magnitude } else { magnitude }
    }
}

/// The exact sum of `xs`, rounded once to a `Float`.
pub fn sum(xs: impl IntoIterator<Item = f64>) -> f64 {
    let mut s = ExactSum::new();
    xs.into_iter().for_each(|x| s.add(x));
    s.value()
}

/// The exact sum of `xs`, rounded once to a `Float32`.
pub fn sum32(xs: impl IntoIterator<Item = f32>) -> f32 {
    let mut s = ExactSum::new();
    xs.into_iter().for_each(|x| s.add(f64::from(x)));
    s.value32()
}

#[cfg(test)]
mod tests;
