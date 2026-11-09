//! `Decimal` in native code, in either form (spec 2.1, decision 0042):
//! the fast 128-bit one, or `BigDecimal` for a value that doesn't fit.
//! Printing, `==`, hashing and ordering reach the big form through hooks
//! set when the first one is made, so other programs don't link its code
//! (decision 0024). Messages must match the interpreter.

use crate::heap::{Data, alloc};
use crate::rt::type_of;
use kavach_runtime_core::{AnyDecimal, BigDecimal, Decimal};
use std::cmp::Ordering;

struct Hooks {
    round: fn(&BigDecimal, u32, bool) -> AnyDecimal,
    show: fn(&BigDecimal) -> String,
    order: fn(&AnyDecimal, &AnyDecimal) -> Ordering,
    to_f64: fn(&AnyDecimal) -> f64,
}

static HOOKS: std::sync::OnceLock<Hooks> = std::sync::OnceLock::new();

fn hooks() -> &'static Hooks {
    HOOKS
        .get()
        .expect("a big decimal was made, so its hooks are set")
}

/// The decimal in object `p`, in its own form.
pub fn any_decimal_of(p: i64) -> AnyDecimal {
    match &*crate::heap::data(p).borrow() {
        Data::BigDecimal(b) => AnyDecimal::Big((**b).clone()),
        Data::Decimal(d) => AnyDecimal::Small(**d),
        _ => AnyDecimal::Small(Decimal::from_int(0)),
    }
}

fn is_big(p: i64) -> bool {
    matches!(&*crate::heap::data(p).borrow(), Data::BigDecimal(_))
}

/// A new object of decimal type `code` holding `d`.
pub(crate) fn new_decimal(d: AnyDecimal, code: i64) -> i64 {
    match d {
        AnyDecimal::Small(d) => alloc(type_of(code), Data::Decimal(Box::new(d))),
        AnyDecimal::Big(b) => {
            HOOKS.get_or_init(|| Hooks {
                round: |b, places, even| AnyDecimal::Big(b.clone()).round(places, even),
                show: |b| b.to_string(),
                order: |a, b| a.compare(b),
                to_f64: |d| d.to_f64(),
            });
            alloc(type_of(code), Data::BigDecimal(Box::new(b)))
        }
    }
}

/// `d.round(places)`, or `round_even` (spec 14.2): a new decimal of type
/// `code`; `p` is borrowed. The 128-bit form rounds without the big one.
pub(crate) fn round(p: i64, places: i64, even: bool, code: i64, line: i32) -> i64 {
    if places < 0 {
        crate::rt::panic_with(
            &format!("can't round to {places} places: give 0 or more"),
            line,
        );
    }
    let places = u32::try_from(places).unwrap_or(u32::MAX);
    let rounded = match &*crate::heap::data(p).borrow() {
        Data::Decimal(d) => AnyDecimal::Small(d.round(places, even)),
        Data::BigDecimal(b) => (hooks().round)(b, places, even),
        _ => AnyDecimal::Small(Decimal::from_int(0)),
    };
    new_decimal(rounded, code)
}

/// Decimal `p` as text.
pub fn show(p: i64) -> String {
    match &*crate::heap::data(p).borrow() {
        Data::BigDecimal(b) => (hooks().show)(b),
        Data::Decimal(d) => d.to_string(),
        _ => String::new(),
    }
}

/// The order of decimals `a` and `b`.
pub fn order(a: i64, b: i64) -> Ordering {
    if is_big(a) || is_big(b) {
        return (hooks().order)(&any_decimal_of(a), &any_decimal_of(b));
    }
    let (x, y) = (crate::display::decimal_of(a), crate::display::decimal_of(b));
    x.partial_cmp(&y).unwrap_or(Ordering::Equal)
}

/// Decimal `p` as the nearest `Float`.
pub fn to_f64(p: i64) -> f64 {
    if is_big(p) {
        return (hooks().to_f64)(&any_decimal_of(p));
    }
    crate::display::decimal_of(p).to_f64()
}
