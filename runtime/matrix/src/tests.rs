//! The product against a plain loop, bit for bit; `fast` within a
//! tolerance; whole numbers that overflow; the other kernels.

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
    assert_eq!(zip(Op::Add, &[250u8], &[10]), Err(Overflow));
    assert_eq!(negate(&[i32::MIN]), Err(Overflow));
}

#[test]
fn item_by_item_sums_and_transposes() {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    assert_eq!(zip(Op::Mul, &a, &a).unwrap()[5], 36.0);
    assert_eq!(scale(&a, 2.0).unwrap()[0], 2.0);
    assert_eq!(transpose(&a, 2, 3), vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    assert_eq!(sums(&a, 2, 3, By::All).unwrap(), vec![21.0]);
    assert_eq!(sums(&a, 2, 3, By::Rows).unwrap(), vec![6.0, 15.0]);
    assert_eq!(sums(&a, 2, 3, By::Cols).unwrap(), vec![5.0, 7.0, 9.0]);
    assert_eq!(
        product::<f64>(&[], &[], (0, 3, 2), false).unwrap(),
        Vec::<f64>::new()
    );
}
