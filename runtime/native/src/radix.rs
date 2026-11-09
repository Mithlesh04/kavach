//! Sorting integers by their digits (radix sort): a few passes over the
//! list instead of `n log n` comparisons.
//!
//! A large list is first split by its top 8-bit digit into 256 buckets,
//! in one pass over memory. Each bucket is then small enough to stay in
//! the cache while its lower digits are sorted, least significant first
//! (each pass keeps the order of the one before). Digits that are the
//! same in every number are skipped.
//!
//! Equal integers can't be told apart, so the result is exactly what any
//! other sort gives (`list.sort()`, spec 14.2).

/// Bits per digit, and the number of different digits: 256 buckets keep
/// one cache line each in the fastest cache while numbers are placed.
const BITS: usize = 8;
const RADIX: usize = 1 << BITS;
const MASK: u64 = RADIX as u64 - 1;

/// Shorter lists are sorted by comparing: counting digits costs more.
const SMALL: usize = 512;

/// Longer lists are split into buckets first (see the top of this file).
const SPLIT: usize = 1 << 16;

/// Sorts integer slots: as `i64` when `signed`, otherwise as `u64`.
pub fn sort(v: &mut [u64], signed: bool) {
    if v.len() < SMALL {
        if signed {
            v.sort_unstable_by_key(|x| *x as i64);
        } else {
            v.sort_unstable();
        }
        return;
    }
    // Flipping the sign bit orders signed numbers as unsigned ones.
    let flip = if signed { 1 << 63 } else { 0 };
    let first = v[0] ^ flip;
    let differ = v.iter().fold(0, |d, x| d | (x ^ flip ^ first));
    // Bits above `bits` are the same in every number.
    let bits = 64 - differ.leading_zeros() as usize;
    let mut other = vec![0u64; v.len()];
    if v.len() < SPLIT || bits <= 2 * BITS {
        if lsd(v, &mut other, bits, flip) {
            v.copy_from_slice(&other);
        }
        return;
    }
    let shift = bits - BITS;
    let mut counts = [0usize; RADIX];
    for x in v.iter() {
        counts[(((x ^ flip) >> shift) & MASK) as usize] += 1;
    }
    place(v, &mut other, &counts, shift, flip);
    // The buckets are independent, so big lists sort them on all cores:
    // the result is the same numbers in the same order either way.
    let len = v.len();
    let mut pairs = Vec::with_capacity(RADIX);
    let (mut rest_b, mut rest_s) = (&mut other[..], &mut v[..]);
    for c in counts {
        let (bucket, rb) = rest_b.split_at_mut(c);
        let (scratch, rs) = rest_s.split_at_mut(c);
        (rest_b, rest_s) = (rb, rs);
        pairs.push((bucket, scratch));
    }
    let workers = match std::env::var_os("KAVACH_SERIAL") {
        Some(_) => 1,
        None => std::thread::available_parallelism().map_or(1, |p| p.get()),
    };
    let one = |pair: &mut (&mut [u64], &mut [u64])| {
        let (bucket, scratch) = pair;
        if !lsd(bucket, scratch, shift, flip) {
            scratch.copy_from_slice(bucket);
        }
    };
    if workers < 2 || len < BIG {
        pairs.iter_mut().for_each(one);
        return;
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let pairs = std::sync::Mutex::new(pairs);
    std::thread::scope(|sc| {
        for _ in 0..workers.min(8) {
            sc.spawn(|| {
                loop {
                    let at = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    // Taking one pair at a time balances the uneven
                    // buckets; the lock guards the list, not the work.
                    let Some(mut pair) = pairs.lock().unwrap().get_mut(at).map(|p| {
                        let (b, s) = p;
                        (std::mem::take(b), std::mem::take(s))
                    }) else {
                        return;
                    };
                    one(&mut pair);
                }
            });
        }
    });
}

/// Lists at least this long sort their buckets on all cores.
const BIG: usize = 1 << 20;

/// Sorts `a` by its lowest `bits` bits, using `b` as room; returns
/// whether the result ended up in `b`.
fn lsd(a: &mut [u64], b: &mut [u64], bits: usize, flip: u64) -> bool {
    let passes = bits.div_ceil(BITS);
    // How many numbers have each digit, for every pass, in one read.
    let mut counts = vec![[0usize; RADIX]; passes];
    for x in a.iter() {
        let key = x ^ flip;
        for (p, c) in counts.iter_mut().enumerate() {
            c[((key >> (p * BITS)) & MASK) as usize] += 1;
        }
    }
    let mut in_b = false;
    for (p, c) in counts.iter().enumerate() {
        if c.contains(&a.len()) {
            continue; // every number has the same digit here
        }
        if in_b {
            place(b, a, c, p * BITS, flip);
        } else {
            place(a, b, c, p * BITS, flip);
        }
        in_b = !in_b;
    }
    in_b
}

/// One pass: copies `from` into `to`, ordered by the digit at `shift`,
/// keeping the order of numbers with the same digit.
fn place(from: &[u64], to: &mut [u64], counts: &[usize; RADIX], shift: usize, flip: u64) {
    let mut at = [0usize; RADIX];
    let mut sum = 0;
    for (a, c) in at.iter_mut().zip(counts) {
        *a = sum;
        sum += c;
    }
    for &x in from {
        let digit = (((x ^ flip) >> shift) & MASK) as usize;
        to[at[digit]] = x;
        at[digit] += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Numbers from a fixed generator, so failures repeat.
    fn numbers(n: usize, seed: u64, spread: u64) -> Vec<u64> {
        let mut x = seed;
        (0..n)
            .map(|_| {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (x >> 7) % spread
            })
            .collect()
    }

    #[test]
    fn sorts_like_the_standard_sort() {
        // Small and large lists; narrow, wide and full ranges; negatives.
        for (n, spread) in [
            (10, 100),
            (600, 3),
            (5000, 1 << 20),
            (5000, u64::MAX),
            (70_000, 1 << 40),
            (70_000, 1 << 12),
            (200_000, u64::MAX),
        ] {
            for signed in [false, true] {
                let mut v = numbers(n, spread.wrapping_add(n as u64), spread);
                if signed {
                    v.iter_mut()
                        .step_by(3)
                        .for_each(|x| *x = (*x as i64).wrapping_neg() as u64);
                }
                let mut expected = v.clone();
                if signed {
                    expected.sort_by_key(|x| *x as i64);
                } else {
                    expected.sort();
                }
                sort(&mut v, signed);
                assert_eq!(v, expected, "n {n}, spread {spread}, signed {signed}");
            }
        }
    }

    #[test]
    fn equal_and_extreme_numbers() {
        let mut same = vec![7u64; 1000];
        sort(&mut same, true);
        assert!(same.iter().all(|x| *x == 7));
        let mut v: Vec<u64> = (0..1000)
            .map(|i| [i64::MIN, i64::MAX, 0, -1][i % 4] as u64)
            .collect();
        sort(&mut v, true);
        assert_eq!(v[0] as i64, i64::MIN);
        assert_eq!(v[999] as i64, i64::MAX);
        assert!(v.windows(2).all(|w| (w[0] as i64) <= (w[1] as i64)));
    }
}
