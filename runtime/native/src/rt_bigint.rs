//! `BigInt` in native code (spec 2.1, decision 0041). It shares the calls
//! of `Decimal` (`rt_numbers.rs`): each one sees which number it holds.
//! Messages must match the interpreter (`compiler/interp/src/bigint.rs`).

use crate::heap::{Data, alloc, release};
use crate::rt::{panic_with, range, type_of};
use kavach_kir::Type;
use kavach_runtime_core::{BigInt, format_float};

/// The number inside object `p`, if it is a `BigInt`.
pub fn big_of(p: i64) -> Option<BigInt> {
    match &*crate::heap::data(p).borrow() {
        Data::BigInt(n) => Some((**n).clone()),
        _ => None,
    }
}

/// What printing, `==`, hashing and ordering need of `BigInt`. Set when
/// the first one is made, so a program without `BigInt` doesn't link its
/// code (decision 0024): every program prints, few use `BigInt`.
struct Hooks {
    show: fn(&BigInt) -> String,
    order: fn(&BigInt, &BigInt) -> std::cmp::Ordering,
    hash: fn(&BigInt) -> u64,
}

static HOOKS: std::sync::OnceLock<Hooks> = std::sync::OnceLock::new();

fn hooks() -> &'static Hooks {
    HOOKS
        .get()
        .expect("a BigInt was made, so its hooks are set")
}

pub(crate) fn new_big(n: BigInt, code: i64) -> i64 {
    HOOKS.get_or_init(|| Hooks {
        show: |n| n.to_string(),
        order: |a, b| a.cmp(b),
        hash: |n| {
            let mut h = crate::hashing::FastHasher::new();
            std::hash::Hash::hash(n, &mut h);
            std::hash::Hasher::finish(&h)
        },
    });
    alloc(type_of(code), Data::BigInt(Box::new(n)))
}

/// `f` given the `BigInt` in object `p` (0 for any other object).
fn with_big<R>(p: i64, f: impl FnOnce(&BigInt) -> R) -> Option<R> {
    match &*crate::heap::data(p).borrow() {
        Data::BigInt(n) => Some(f(n)),
        _ => None,
    }
}

/// `BigInt` `p` as its digits.
pub fn show(p: i64) -> String {
    with_big(p, hooks().show).unwrap_or_default()
}

/// The order of `BigInt`s `a` and `b`, or `None` when `a` isn't one.
pub fn order(a: i64, b: i64) -> Option<std::cmp::Ordering> {
    with_big(a, |x| with_big(b, |y| (hooks().order)(x, y)))?
}

/// A hash of `BigInt` `p` that agrees with `==`.
pub fn hash(p: i64) -> u64 {
    with_big(p, hooks().hash).unwrap_or_default()
}

/// `+ - * / %` (`op` 0 to 4) on two `BigInt`s, both released.
pub(crate) fn arith(op: i64, x: BigInt, y: BigInt, code: i64, line: i32) -> i64 {
    let zero = BigInt::default();
    let n = match op {
        0 => x + y,
        1 => x - y,
        2 => x * y,
        3 | 4 if y == zero => panic_with("division by zero", line),
        3 => x / y,
        _ => x % y,
    };
    new_big(n, code)
}

/// `a ** b` on `BigInt`s (both released).
pub(crate) fn pow(a: i64, b: i64, code: i64, line: i32) -> i64 {
    let (x, y) = (big_of(a).unwrap_or_default(), big_of(b).unwrap_or_default());
    release(a);
    release(b);
    match kavach_runtime_core::big_pow(&x, &y) {
        Ok(n) => new_big(n, code),
        Err(message) => panic_with(&message, line),
    }
}

/// `BigInt(x)` for a number in `slot` of type `from` (a `BigInt` or a
/// `Decimal` held by reference), cut to a whole one as `Int(x)` does.
pub(crate) fn from_slot(slot: i64, from: Type, code: i64, line: i32) -> i64 {
    let (slot, from) = match from {
        Type::F32 => (
            f64::from(crate::display::f32_of(slot)).to_bits() as i64,
            Type::F64,
        ),
        _ => (slot, from),
    };
    let n = if from == Type::F64 {
        let f = f64::from_bits(slot as u64);
        let parsed = f
            .is_finite()
            .then(|| format!("{:.0}", f.trunc()).parse().ok());
        parsed.flatten().unwrap_or_else(|| {
            panic_with(&format!("{} doesn't fit in BigInt", format_float(f)), line)
        })
    } else if from.is_ref() {
        let n = big_of(slot).unwrap_or_else(|| crate::rt_big_decimal::any_decimal_of(slot).trunc());
        release(slot);
        n
    } else if from.is_signed() {
        BigInt::from(slot)
    } else {
        BigInt::from(slot as u64)
    };
    new_big(n, code)
}

/// `Int(n)`, `Float(n)` and friends for a `BigInt` `n`; panics when it
/// doesn't fit.
pub(crate) fn to_slot(n: &BigInt, to: Type, line: i32) -> i64 {
    if to == Type::F32 {
        let f: f32 = n.to_string().parse().unwrap_or(f32::INFINITY);
        return crate::display::f32_slot(f);
    }
    if to == Type::F64 {
        let f: f64 = n.to_string().parse().unwrap_or(f64::INFINITY);
        return f.to_bits() as i64;
    }
    let (lo, hi) = range(to);
    match i128::try_from(n) {
        Ok(v) if (lo..=hi).contains(&v) => v as i64,
        _ => panic_with(&format!("{n} doesn't fit in {}", to.kavach_name()), line),
    }
}
