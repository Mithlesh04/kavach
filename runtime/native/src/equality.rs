// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Equality, ordering and hashing of values in native code (spec 1.8,
//! 2.2): structural `==`, map keys and set items where NaN equals NaN,
//! and a hash that agrees with equality. Split from `display.rs`.

use crate::display::{boxed, f32_of};
use crate::hashing::FastHasher;
use crate::heap::{Data, items_of, obj, shape, with_str};
use kavach_kir::{Shape, Type};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

thread_local! {
    /// Whether `equal` is comparing map keys or set items, where NaN
    /// equals NaN (spec 2.2); `==` keeps IEEE's NaN, equal to nothing.
    static KEYS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether `a` and `b` are the same key of a map or item of a set (spec
/// 2.2): equal, with any NaN, however deep, equal to NaN.
pub fn same_key(a: i64, b: i64, ty: Type) -> bool {
    let before = KEYS.with(|k| k.replace(true));
    let same = equal(a, b, ty);
    KEYS.with(|k| k.set(before));
    same
}

/// Two floats equal: by value, NaN equal to NaN only as keys.
fn same_float(x: f64, y: f64) -> bool {
    x == y || (x.is_nan() && y.is_nan() && KEYS.with(std::cell::Cell::get))
}

/// Structural equality (spec 1.8), without consuming either value.
pub fn equal(a: i64, b: i64, ty: Type) -> bool {
    match ty {
        Type::F64 => same_float(f64::from_bits(a as u64), f64::from_bits(b as u64)),
        Type::F32 => same_float(f64::from(f32_of(a)), f64::from(f32_of(b))),
        Type::Ref(_) => {
            // One value in memory (a copy not yet changed, or a part two
            // values share) is equal to itself: no need to look inside
            // (decision 0039).
            if a == b {
                return true;
            }
            if a == 0 || b == 0 {
                return false;
            }
            match shape(ty) {
                // Interface values: the same concrete type, equal values.
                Shape::Iface { .. } => {
                    let (x, y) = (items_of(a), items_of(b));
                    x[1] == y[1] && equal(x[0] as i64, y[0] as i64, crate::rt::type_of(x[1] as i64))
                }
                Shape::Optional(inner) if inner.is_ref() => equal(a, b, *inner),
                Shape::Optional(inner) => equal(boxed(a), boxed(b), *inner),
                Shape::Str => with_str(a, |x| with_str(b, |y| x == y)),
                Shape::Bytes => {
                    crate::rt_bytes::with_bytes(a, |x| crate::rt_bytes::with_bytes(b, |y| x == y))
                }
                Shape::Decimal => crate::rt_big_decimal::order(a, b) == Ordering::Equal,
                Shape::BigInt => crate::rt_bigint::order(a, b) == Some(Ordering::Equal),
                Shape::Regex => crate::opaque::describe(a) == crate::opaque::describe(b),
                // A number in a type (spec 2.9) is no value's type.
                Shape::Num(_) | Shape::Whole { .. } => false,
                // Copies of one cell are the same value (`send.rs`).
                Shape::Shared(_)
                | Shape::Task(_)
                | Shape::Channel(_)
                | Shape::Reader(_)
                | Shape::Socket => crate::send::identity(a) == crate::send::identity(b),
                Shape::Error | Shape::Func(..) | Shape::Closure(_) => a == b,
                // Maps and sets: the same keys (with equal values), in any
                // order, each looked up by hash (spec 1.8).
                Shape::Set(t) => same_entries(a, b, *t, |_, _| true),
                Shape::Map(k, v) => same_entries(a, b, *k, |x, y| equal(x, y, *v)),
                Shape::List(t) => with_items(a, |x| {
                    with_items(b, |y| {
                        // Plain numbers: equal exactly when their slots are.
                        x.len() == y.len()
                            && if t.is_ref() || t.is_float() {
                                x.iter()
                                    .zip(y)
                                    .all(|(p, q)| equal(*p as i64, *q as i64, *t))
                            } else {
                                x == y
                            }
                    })
                }),
                Shape::Tuple(ts) => with_items(a, |x| {
                    with_items(b, |y| {
                        x.iter()
                            .zip(y)
                            .zip(ts)
                            .all(|((p, q), t)| equal(*p as i64, *q as i64, *t))
                    })
                }),
                Shape::Record { fields, .. } => with_items(a, |x| {
                    with_items(b, |y| {
                        x.iter()
                            .zip(y)
                            .zip(fields)
                            .all(|((p, q), f)| equal(*p as i64, *q as i64, f.ty))
                    })
                }),
                // A `Json`, maybe read lazily from its text (2.12).
                Shape::Union { .. } if crate::rt_json::is_json(ty) => crate::rt_json::same(a, b),
                Shape::Union { variants, .. } => {
                    let (va, vb) = (crate::heap::tag_of(a), crate::heap::tag_of(b));
                    va == vb
                        && with_items(a, |x| {
                            with_items(b, |y| {
                                x.iter()
                                    .zip(y)
                                    .zip(&variants[va as usize].fields)
                                    .all(|((p, q), f)| equal(*p as i64, *q as i64, f.ty))
                            })
                        })
                }
            }
        }
        _ => a == b,
    }
}

/// `f` given the items of `p` as `items_of` gives them, read in place
/// when its slots hold them as they are: no copy.
fn with_items<R>(p: i64, f: impl FnOnce(&[u64]) -> R) -> R {
    let ty = obj(p).ty();
    if crate::flat::packed(ty) || crate::flat::width(ty) > 0 {
        f(&items_of(p))
    } else {
        f(crate::heap::slots(p))
    }
}

/// Whether maps (or sets) `a` and `b`, with keys of type `kt`, hold the
/// same keys, their values `same`: each key of `a` looked up in `b`.
fn same_entries(a: i64, b: i64, kt: Type, same: impl Fn(i64, i64) -> bool) -> bool {
    let size = |p: i64| match &*crate::heap::data(p).borrow() {
        Data::Map(t) => t.len(),
        _ => 0,
    };
    if size(a) != size(b) {
        return false;
    }
    let value_in_b = |e: usize| match &*crate::heap::data(b).borrow() {
        Data::Map(t) => t.slots[e * 2 + 1] as i64,
        _ => 0,
    };
    match &*crate::heap::data(a).borrow() {
        Data::Map(t) => t.slots.chunks(2).all(|entry| {
            crate::rt_maps::find(b, entry[0] as i64, kt)
                .is_some_and(|e| same(entry[1] as i64, value_in_b(e)))
        }),
        _ => true,
    }
}

/// Ordering of text, decimals, tuples, lists, rings and `Bytes` (spec 1.8).
pub fn compare(a: i64, b: i64) -> Ordering {
    // Lists and rings item by item, the shorter first when one runs out;
    // `Bytes` byte by byte.
    match shape(obj(a).ty()) {
        Shape::List(t) => {
            let t = *t;
            return with_items(a, |x| {
                with_items(b, |y| {
                    x.iter()
                        .zip(y)
                        .map(|(p, q)| crate::rt_text::order(*p as i64, *q as i64, t))
                        .find(|o| o.is_ne())
                        .unwrap_or_else(|| x.len().cmp(&y.len()))
                })
            });
        }
        Shape::Bytes => {
            use crate::rt_bytes::with_bytes;
            return with_bytes(a, |x| with_bytes(b, |y| x.cmp(y)));
        }
        _ => {}
    }
    // Tuples order item by item, the first that differs deciding (1.8).
    if let Shape::Tuple(ts) = shape(obj(a).ty()) {
        return with_items(a, |x| {
            with_items(b, |y| {
                x.iter()
                    .zip(y)
                    .zip(ts)
                    .map(|((p, q), t)| crate::rt_text::order(*p as i64, *q as i64, *t))
                    .find(|o| o.is_ne())
                    .unwrap_or(Ordering::Equal)
            })
        });
    }
    if matches!(
        &*crate::heap::data(a).borrow(),
        Data::Decimal(_) | Data::BigDecimal(_)
    ) {
        return crate::rt_big_decimal::order(a, b);
    }
    if let Some(order) = crate::rt_bigint::order(a, b) {
        return order;
    }
    with_str(a, |x| with_str(b, |y| x.cmp(y)))
}

/// A hash that agrees with `equal`: equal values hash the same.
pub fn hash_slot(slot: i64, ty: Type) -> u64 {
    let mut h = FastHasher::new();
    hash_into(slot, ty, &mut h);
    h.finish()
}

fn hash_into(slot: i64, ty: Type, h: &mut FastHasher) {
    match ty {
        // 0.0 and -0.0 are equal, so they hash the same; and every NaN,
        // equal as a key (spec 2.2).
        Type::F64 => float_bits(f64::from_bits(slot as u64)).hash(h),
        Type::F32 => float_bits(f64::from(f32_of(slot))).hash(h),
        Type::Ref(_) if slot == 0 => 0u8.hash(h),
        Type::Ref(_) => match shape(ty) {
            Shape::Iface { .. } => {
                let s = items_of(slot);
                hash_into(s[0] as i64, crate::rt::type_of(s[1] as i64), h);
            }
            Shape::Optional(inner) if inner.is_ref() => hash_into(slot, *inner, h),
            Shape::Optional(inner) => hash_into(boxed(slot), *inner, h),
            Shape::Str => with_str(slot, |s| s.hash(h)),
            Shape::Bytes => crate::rt_bytes::with_bytes(slot, |b| b.hash(h)),
            // Equal decimals (1.10 and 1.1) have the same value as a float.
            Shape::Decimal => crate::rt_big_decimal::to_f64(slot).to_bits().hash(h),
            Shape::BigInt => crate::rt_bigint::hash(slot).hash(h),
            Shape::Regex => crate::opaque::describe(slot).hash(h),
            Shape::Num(_) | Shape::Whole { .. } => {}
            Shape::Shared(_)
            | Shape::Task(_)
            | Shape::Channel(_)
            | Shape::Reader(_)
            | Shape::Socket => crate::send::identity(slot).hash(h),
            Shape::Error | Shape::Func(..) | Shape::Closure(_) => slot.hash(h),
            Shape::Set(t) => items_of(slot)
                .iter()
                .step_by(2)
                .fold(0u64, |acc, s| acc.wrapping_add(hash_slot(*s as i64, *t)))
                .hash(h),
            // A flat list's items are boxed by `items_of`, so the item type
            // is taken from the shape, not from the slots.
            Shape::List(t) => {
                for s in &items_of(slot) {
                    hash_into(*s as i64, *t, h);
                }
            }
            // Equal maps may hold their entries in another order.
            Shape::Map(k, v) => items_of(slot)
                .chunks(2)
                .fold(0u64, |acc, e| {
                    let mut eh = FastHasher::new();
                    hash_into(e[0] as i64, *k, &mut eh);
                    hash_into(e[1] as i64, *v, &mut eh);
                    acc.wrapping_add(eh.finish())
                })
                .hash(h),
            Shape::Tuple(_) | Shape::Record { .. } => {
                let types = crate::heap::slot_types(slot);
                for (s, t) in items_of(slot).iter().zip(types) {
                    hash_into(*s as i64, t, h);
                }
            }
            Shape::Union { variants, .. } => {
                let (variant, items) = (crate::heap::tag_of(slot) as usize, items_of(slot));
                variant.hash(h);
                for (s, f) in items.iter().zip(&variants[variant].fields) {
                    hash_into(*s as i64, f.ty, h);
                }
            }
        },
        _ => slot.hash(h),
    }
}

/// A float's bits for hashing: one for 0.0 and -0.0, one for every NaN.
fn float_bits(x: f64) -> u64 {
    if x.is_nan() {
        f64::NAN.to_bits()
    } else {
        (x + 0.0).to_bits()
    }
}
