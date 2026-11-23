// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

use super::{ExactSum, sum, sum32};

/// Python's `math.fsum` (Shewchuk's partials, then the half-way
/// correction): correctly rounded while nothing overflows.
fn fsum(xs: &[f64]) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for &x0 in xs {
        let mut x = x0;
        let mut i = 0;
        for j in 0..partials.len() {
            let mut y = partials[j];
            if x.abs() < y.abs() {
                std::mem::swap(&mut x, &mut y);
            }
            let hi = x + y;
            let lo = y - (hi - x);
            if lo != 0.0 {
                partials[i] = lo;
                i += 1;
            }
            x = hi;
        }
        partials.truncate(i);
        partials.push(x);
    }
    let Some(mut k) = partials.len().checked_sub(1) else {
        return 0.0;
    };
    let (mut hi, mut lo) = (partials[k], 0.0);
    while k > 0 {
        k -= 1;
        let (x, y) = (hi, partials[k]);
        hi = x + y;
        lo = y - (hi - x);
        if lo != 0.0 {
            break;
        }
    }
    if k > 0 && ((lo < 0.0 && partials[k - 1] < 0.0) || (lo > 0.0 && partials[k - 1] > 0.0)) {
        let y = lo * 2.0;
        let x = hi + y;
        if y == x - hi {
            hi = x;
        }
    }
    hi
}

/// A simple generator, so the tests are the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }

    /// A float with an exponent in `-spread..spread` and either sign.
    fn float(&mut self, spread: i32) -> f64 {
        let m = (self.next() % (1 << 52)) as f64 / (1u64 << 52) as f64 + 1.0;
        let e = (self.next() % (2 * spread as u64)) as i32 - spread;
        let sign = if self.next().is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        sign * libm::scalbn(m, e)
    }
}

fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[test]
fn sums_are_exact_and_rounded_once() {
    let two = |n: i32| libm::scalbn(1.0, n);
    let cases: &[(&[f64], f64)] = &[
        (&[1e20, 1.0, -1e20], 1.0),
        (&[], 0.0),
        (&[-0.0, -0.0], -0.0),
        (&[-0.0, 0.0], 0.0),
        (&[1.0, -1.0], 0.0),
        (&[f64::INFINITY, 1.0], f64::INFINITY),
        (&[f64::NEG_INFINITY, 1.0], f64::NEG_INFINITY),
        (&[f64::INFINITY, f64::NEG_INFINITY], f64::NAN),
        (&[f64::NAN, 1.0], f64::NAN),
        (&[f64::MAX, f64::MAX], f64::INFINITY),
        (&[-f64::MAX, -f64::MAX], f64::NEG_INFINITY),
        (&[f64::MAX, f64::MAX, -f64::MAX], f64::MAX),
        (&[5e-324, 5e-324], 1e-323),
        (&[5e-324, -5e-324, -0.0], 0.0),
        (&[0.1, 0.2, 0.3], 0.6),
        (&[1.0, two(-53)], 1.0),
        (&[1.0, two(-53), two(-100)], 1.0 + two(-52)),
        (&[1.0 + two(-52), two(-53)], 1.0 + two(-51)),
        (&[two(-1022), -5e-324], two(-1022) - 5e-324),
    ];
    for (xs, want) in cases {
        let got = sum(xs.iter().copied());
        assert!(same(got, *want), "{xs:?}: {got:?}, not {want:?}");
    }
}

#[test]
fn float32_sums_round_once_to_float32() {
    let two = |n: i32| libm::scalbnf(1.0, n);
    let cases: &[(&[f32], f32)] = &[
        (&[1e20, 1.0, -1e20], 1.0),
        (&[0.1; 10], 1.0),
        (&[f32::MAX, f32::MAX], f32::INFINITY),
        (&[f32::MAX, f32::MAX, -f32::MAX], f32::MAX),
        (&[1e-45, 1e-45], 3e-45),
        (&[-0.0], -0.0),
        // Rounded to a `Float` first, this would be a tie, and 1.0.
        (&[1.0, two(-24), two(-80)], 1.0 + two(-23)),
    ];
    for (xs, want) in cases {
        let got = sum32(xs.iter().copied());
        assert!(
            got.to_bits() == want.to_bits(),
            "{xs:?}: {got:?}, not {want:?}"
        );
    }
}

#[test]
fn any_order_gives_the_same_bits_as_a_correct_rounding() {
    let mut g = Lcg(7);
    for round in 0..300 {
        let n = 1 + (g.next() % 60) as usize;
        let spread = [4, 60, 300][round % 3];
        let mut xs: Vec<f64> = (0..n).map(|_| g.float(spread)).collect();
        if round % 5 == 0 {
            // Cancellation: every value and its negation, and one more.
            let more: Vec<f64> = xs.iter().map(|x| -x).collect();
            xs.extend(more);
            xs.push(g.float(spread));
        }
        let want = sum(xs.iter().copied());
        assert!(same(want, fsum(&xs)), "{xs:?}");
        for _ in 0..4 {
            for i in (1..xs.len()).rev() {
                xs.swap(i, (g.next() % (i as u64 + 1)) as usize);
            }
            assert!(same(sum(xs.iter().copied()), want));
        }
    }
}

#[test]
fn float32_sums_match_a_float_sum_with_room_to_spare() {
    // Exponents in -10..10: every sum of these fits a `Float` exactly,
    // so rounding that to a `Float32` is rounding once.
    let mut g = Lcg(11);
    for _ in 0..300 {
        let n = 1 + (g.next() % 40) as usize;
        let xs: Vec<f32> = (0..n).map(|_| g.float(10) as f32).collect();
        let exact: f64 = xs.iter().map(|&x| f64::from(x)).sum();
        assert_eq!(
            sum32(xs.iter().copied()).to_bits(),
            (exact as f32).to_bits()
        );
    }
}

#[test]
fn merged_sums_are_one_sum() {
    let mut g = Lcg(3);
    let xs: Vec<f64> = (0..200).map(|_| g.float(200)).collect();
    let (mut a, mut b) = (ExactSum::new(), ExactSum::new());
    xs[..77].iter().for_each(|&x| a.add(x));
    xs[77..].iter().for_each(|&x| b.add(x));
    a.merge(&b);
    assert!(same(a.value(), sum(xs.iter().copied())));
}

#[test]
fn full_bins_are_moved_into_the_pieces() {
    // Each value's bits are near 2^53, so a bin fills after 512 of them.
    let mut xs = vec![1.9999999999999998; 3000];
    xs.extend(vec![-1.9999999999999998e-300; 1500]);
    xs.push(3.0e-310);
    assert!(same(sum(xs.iter().copied()), fsum(&xs)));
}
