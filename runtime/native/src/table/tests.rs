use super::*;

#[test]
fn finds_what_was_pushed_and_keeps_order_after_remove() {
    let mut t = Table::default();
    for k in 0..1000u64 {
        t.push(k.wrapping_mul(K1) >> 3, k, k * 2);
    }
    for k in 0..1000u64 {
        assert_eq!(
            t.find(k.wrapping_mul(K1) >> 3, |x| x == k),
            Some(k as usize)
        );
    }
    assert_eq!(t.remove(10), (10, 20));
    assert_eq!(t.find(10u64.wrapping_mul(K1) >> 3, |x| x == 10), None);
    assert_eq!(t.find(11u64.wrapping_mul(K1) >> 3, |x| x == 11), Some(10));
    assert_eq!(t.slots[20], 11);
}

#[test]
fn sequential_integer_keys_keep_the_cheap_hash() {
    let mut t = Table::default();
    for k in 0..100_000u64 {
        t.insert_int(k, k + 1);
    }
    assert!(!t.strong);
    assert!((0..100_000u64).all(|k| t.find_int(k) == Some(k as usize)));
    assert_eq!(t.find_int(100_000), None);
    assert!(!t.wants_strong());
}

#[test]
fn colliding_integer_keys_switch_to_the_strong_hash() {
    // Keys equal modulo every power of two up to 2^40 all start at the
    // same position under the cheap hash.
    let mut t = Table::default();
    for k in 0..1000u64 {
        t.insert_int(k << 40, k);
    }
    assert!(t.strong);
    assert!((0..1000u64).all(|k| t.find_int(k << 40) == Some(k as usize)));
    assert_eq!(t.slots[2 * 999], 999 << 40, "order is kept");
}

#[test]
fn a_long_run_found_by_a_lookup_switches_too() {
    // Keys `k` and `k + half` share a position. 256 such pairs fill
    // one aligned block of positions completely, whatever the seed,
    // so the block becomes a single run of 512 entries.
    let mut t = Table::default();
    let half = 1u64 << 30;
    for k in 0..256u64 {
        t.insert_int(k, 0);
        t.insert_int(k + half, 0);
    }
    // Missing keys that start at every position of the run: some of
    // them probe far, unless inserting already switched the hash.
    for k in 0..256u64 {
        assert_eq!(t.find_int(k + 2 * half), None);
    }
    assert!(t.strong || t.wants_strong());
    t.make_strong();
    assert!((0..256u64).all(|k| t.find_int(k).is_some() && t.find_int(k + half).is_some()));
}

/// Doubling the index in place (`grow.rs`) keeps every key findable and
/// the entries in the order they came, whatever the keys' pattern.
#[test]
fn integer_keys_survive_doubling_in_place() {
    let mut random = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        random
    };
    let n = 40_000u64;
    let patterns: Vec<(&str, Vec<u64>)> = vec![
        ("up", (0..n).collect()),
        ("down", (0..n).rev().collect()),
        ("stride 1024", (0..n).map(|k| k * 1024).collect()),
        (
            "negative",
            (0..n).map(|k| (-(k as i64) - 1) as u64).collect(),
        ),
        ("random", (0..n).map(|_| next()).collect()),
    ];
    for (name, keys) in patterns {
        let mut t = Table::default();
        for (i, &k) in keys.iter().enumerate() {
            t.insert_int(k, i as u64);
        }
        for (i, &k) in keys.iter().enumerate() {
            assert_eq!(t.get_int(k), Some(i as u64), "{name}: key {k}");
            assert_eq!(t.find_int(k), Some(i), "{name}: entry of {k}");
            assert_eq!(t.slots[2 * i], k, "{name}: order");
        }
    }
}

/// A key that would wrap past the end of the doubled index: every key is
/// placed again instead. With seed 0 the cheap hash is `key * 2`, so keys
/// 15, 31 and 47 share a position and run past the end of each size.
#[test]
fn a_run_wrapping_past_the_end_still_doubles_right() {
    let mut t = Table {
        seed: 0,
        ..Table::default()
    };
    let keys = [15u64, 31, 47, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12];
    for (i, &k) in keys.iter().enumerate() {
        t.insert_int(k, i as u64 + 100);
        for (j, &seen) in keys[..=i].iter().enumerate() {
            assert_eq!(t.get_int(seen), Some(j as u64 + 100), "{seen} after {k}");
        }
    }
    assert_eq!(t.get_int(63), None);
}

#[test]
fn the_empty_marker_key_still_works_as_a_key() {
    let mut t = Table::default();
    for k in 0..100u64 {
        t.insert_int(k, k + 1);
    }
    let marker = empty();
    assert_eq!(t.get_int(marker), None);
    t.insert_int(marker, 7);
    assert_eq!(t.get_int(marker), Some(7));
    assert_eq!(t.insert_int(marker, 9), Some(7));
    assert_eq!(t.get_int(marker), Some(9));
    let e = t.find_int(marker).expect("stored");
    assert_eq!(t.remove(e), (marker, 9));
    assert_eq!(t.get_int(marker), None);
    assert!((0..100u64).all(|k| t.get_int(k) == Some(k + 1)));
}

#[test]
fn equal_hashes_are_told_apart_by_key() {
    let mut t = Table::default();
    for k in 0..50u64 {
        t.push(7, k, k);
    }
    assert_eq!(t.find(7, |x| x == 42), Some(42));
    assert_eq!(t.find(7, |x| x == 99), None);
}

#[test]
fn small_tables_find_keys_without_an_index_and_grow_into_one() {
    // Keys 0..20 with hashes that collide in pairs.
    let mut t = Table::default();
    for k in 0..20u64 {
        t.push(k / 2, k, k * 10);
        for j in 0..=k {
            assert_eq!(t.find(j / 2, |x| x == j), Some(j as usize), "{j} after {k}");
        }
        assert_eq!(t.find(99, |_| true), None);
    }
    let mut small = Table::with_capacity(3);
    for k in 0..3u64 {
        small.push(k, k, k);
    }
    small.remove(1);
    assert_eq!(small.find(2, |x| x == 2), Some(1));
    assert_eq!(small.find(1, |x| x == 1), None);
}

#[test]
fn a_hint_finds_at_once_and_a_wrong_one_still_finds() {
    for n in [3u64, 1000] {
        let mut t = Table::default();
        for k in 0..n {
            t.push(k.wrapping_mul(K1) >> 3, k, k * 2);
        }
        let h = |k: u64| k.wrapping_mul(K1) >> 3;
        // Right, wrong, past the end, and two keys sharing a hash.
        assert_eq!(t.find_from(2, h(2), |x| x == 2), Some(2));
        assert_eq!(t.find_from(0, h(2), |x| x == 2), Some(2));
        assert_eq!(t.find_from(usize::MAX, h(1), |x| x == 1), Some(1));
        assert_eq!(t.find_from(1, h(1), |x| x == 99_999), None);
    }
}
