// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The product against a plain loop, bit for bit; `fast` within a
//! tolerance; whole numbers that overflow; empty sizes.

use super::*;

/// A plain loop: each item's products added in order, `k` from 0 up.
fn by_hand(a: &[f64], b: &[f64], n: usize, m: usize, p: usize) -> Vec<f64> {
    let mut out = vec![0.0; n * p];
    for i in 0..n {
        for j in 0..p {
            let mut sum = 0.0;
            for k in 0..m {
                sum += a[i * m + k] * b[k * p + j];
            }
            out[i * p + j] = sum;
        }
    }
    out
}

/// Numbers whose sums round differently in other orders.
fn numbers(n: usize, seed: u64) -> Vec<f64> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((x >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 1e3
        })
        .collect()
}

#[test]
fn the_product_is_the_plain_loop_bit_for_bit() {
    // Small, and big enough to be spread over threads.
    for (n, m, p) in [(1, 1, 1), (3, 4, 5), (7, 1, 3), (130, 97, 211)] {
        let (a, b) = (numbers(n * m, 1), numbers(m * p, 2));
        let got = product(&a, &b, (n, m, p), false).unwrap();
        let want = by_hand(&a, &b, n, m, p);
        assert!(
            got.iter()
                .zip(&want)
                .all(|(x, y)| x.to_bits() == y.to_bits()),
            "{n}×{m}×{p}"
        );
    }
}

#[test]
fn fast_is_close_to_the_exact_product() {
    let (n, m, p) = (150, 120, 170);
    let (a, b) = (numbers(n * m, 3), numbers(m * p, 4));
    let exact = product(&a, &b, (n, m, p), false).unwrap();
    let fast = product(&a, &b, (n, m, p), true).unwrap();
    for (x, y) in exact.iter().zip(&fast) {
        assert!((x - y).abs() <= 1e-9 * (1.0 + x.abs()), "{x} {y}");
    }
    let small: Vec<f32> = a.iter().map(|x| *x as f32).collect();
    let other: Vec<f32> = b.iter().map(|x| *x as f32).collect();
    let fast32 = product(&small, &other, (n, m, p), true).unwrap();
    let exact32 = product(&small, &other, (n, m, p), false).unwrap();
    for (x, y) in exact32.iter().zip(&fast32) {
        assert!((x - y).abs() <= 1e-2 * (1.0 + x.abs()), "{x} {y}");
    }
}

#[test]
fn whole_numbers_say_when_they_overflow() {
    assert_eq!(product(&[2i64, 3], &[4, 5], (1, 2, 1), true), Ok(vec![23]));
    assert_eq!(
        product(&[i64::MAX, 1], &[2, 1], (1, 2, 1), false),
        Err(Overflow)
    );
    assert_eq!(product(&[100i8], &[2], (1, 1, 1), false), Err(Overflow));
}

#[test]
fn a_product_with_no_rows_is_empty() {
    assert_eq!(
        product::<f64>(&[], &[], (0, 3, 2), false).unwrap(),
        Vec::<f64>::new()
    );
}
