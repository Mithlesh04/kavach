//! Run-time functions for numbers beyond plain machine arithmetic:
//! `Decimal` values, `wrapping_*` and `checked_*` methods, and `parse`.
//! Messages must match the interpreter (`compiler/interp/src/ops.rs`).

use crate::heap::{Data, alloc, new_str, release, shape, str_of};
use crate::rt::{panic_with, range, text, type_of};
use crate::rt_big_decimal::any_decimal_of;
use crate::rt_data::optional;
use crate::rt_errors::set_error;
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::{AnyDecimal, Decimal, format_float};

/// A slot holding an integer of type `t`, as an exact number.
fn int_value(slot: i64, t: Type) -> i128 {
    if t.is_signed() {
        slot as i128
    } else {
        slot as u64 as i128
    }
}

/// `wrapping_*` and `checked_*` on integers (spec 5.6). `code` is the
/// result type: the integer type, or an optional of it.
pub fn int_method(op: RtOp, a: i64, b: i64, code: i64) -> i64 {
    let result = type_of(code);
    let t = match shape(result) {
        Shape::Optional(inner) if result.is_ref() => *inner,
        _ => result,
    };
    let (n, m) = (int_value(a, t), int_value(b, t));
    let exact = match op {
        RtOp::WrappingAdd | RtOp::CheckedAdd => n.checked_add(m),
        RtOp::WrappingSub | RtOp::CheckedSub => n.checked_sub(m),
        RtOp::WrappingMul | RtOp::CheckedMul => n.checked_mul(m),
        _ if m == 0 => None,
        _ => n.checked_div(m),
    };
    let (lo, hi) = range(t);
    if matches!(
        op,
        RtOp::WrappingAdd | RtOp::WrappingSub | RtOp::WrappingMul
    ) {
        let wrapped = exact.map_or(0, |x| (x - lo).rem_euclid(hi - lo + 1) + lo);
        return wrapped as i64;
    }
    let fits = exact.filter(|x| (lo..=hi).contains(x));
    optional(fits.map(|x| x as i64), code)
}

fn new_decimal(d: Decimal, code: i64) -> i64 {
    alloc(type_of(code), Data::Decimal(Box::new(d)))
}

/// A `Decimal` literal, from its text.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_dec_new(ptr: *const u8, len: usize, code: i64) -> i64 {
    if shape(type_of(code)) == &Shape::BigInt {
        let n = text(ptr, len).parse().unwrap_or_default();
        return crate::rt_bigint::new_big(n, code);
    }
    let d = AnyDecimal::parse(text(ptr, len)).unwrap_or(AnyDecimal::Small(Decimal::from_int(0)));
    crate::rt_big_decimal::new_decimal(d, code)
}

/// `+ - * / %` on decimals: `op` is 0 to 4 in that order.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_dec_arith(op: i64, a: i64, b: i64, code: i64, line: i32) -> i64 {
    if let Some(x) = crate::rt_bigint::big_of(a) {
        let y = crate::rt_bigint::big_of(b).unwrap_or_default();
        release(a);
        release(b);
        return crate::rt_bigint::arith(op, x, y, code, line);
    }
    let (x, y) = (any_decimal_of(a), any_decimal_of(b));
    release(a);
    release(b);
    let d = match op {
        0 => x.add(&y),
        1 => x.sub(&y),
        2 => x.mul(&y),
        3 => x
            .div(&y)
            .unwrap_or_else(|| panic_with("division by zero", line)),
        _ => panic_with(
            "`%` works on whole numbers and floats, not on `Decimal`",
            line,
        ),
    };
    crate::rt_big_decimal::new_decimal(d, code)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_dec_neg(a: i64, code: i64) -> i64 {
    if let Some(n) = crate::rt_bigint::big_of(a) {
        release(a);
        return crate::rt_bigint::new_big(-n, code);
    }
    let d = any_decimal_of(a).neg();
    release(a);
    crate::rt_big_decimal::new_decimal(d, code)
}

/// `Decimal(x)` for an integer or float `x` of type `from`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_dec_from(slot: i64, from: i64, code: i64, line: i32) -> i64 {
    let from = type_of(from);
    // A `Float32` widens exactly; it converts as that `Float` does.
    let (slot, from) = match from {
        Type::F32 => (
            f64::from(crate::display::f32_of(slot)).to_bits() as i64,
            Type::F64,
        ),
        _ => (slot, from),
    };
    if shape(type_of(code)) == &Shape::BigInt {
        return crate::rt_bigint::from_slot(slot, from, code, line);
    }
    let d = if let Some(n) = from
        .is_ref()
        .then(|| crate::rt_bigint::big_of(slot))
        .flatten()
    {
        release(slot);
        let d = AnyDecimal::parse(&n.to_string());
        return crate::rt_big_decimal::new_decimal(
            d.unwrap_or(AnyDecimal::Small(Decimal::from_int(0))),
            code,
        );
    } else if from == Type::F64 {
        let f = f64::from_bits(slot as u64);
        Decimal::parse(&f.to_string()).unwrap_or_else(|| {
            panic_with(&format!("{} doesn't fit in Decimal", format_float(f)), line)
        })
    } else {
        Decimal::from_int(int_value(slot, from))
    };
    new_decimal(d, code)
}

/// `Int(d)`, `Float(d)` and friends for a decimal `p`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_dec_to(p: i64, to: i64, line: i32) -> i64 {
    if let Some(n) = crate::rt_bigint::big_of(p) {
        release(p);
        return crate::rt_bigint::to_slot(&n, type_of(to), line);
    }
    let to = type_of(to);
    if to == Type::F32 {
        // Read from its digits: rounded once.
        let f: f32 = any_decimal_of(p)
            .to_string()
            .parse()
            .unwrap_or(f32::INFINITY);
        release(p);
        return crate::display::f32_slot(f);
    }
    if to == Type::F64 {
        let f = crate::rt_big_decimal::to_f64(p);
        release(p);
        return f.to_bits() as i64;
    }
    let d = any_decimal_of(p);
    release(p);
    let (lo, hi) = range(to);
    match i128::try_from(&d.trunc()) {
        Ok(n) if (lo..=hi).contains(&n) => n as i64,
        _ => panic_with(&format!("{d} doesn't fit in {}", to.kavach_name()), line),
    }
}

/// `Int(text)` and friends (spec 2.1). On bad text it returns 0 and
/// leaves the error for `kv_take_error`. `str_code` is the type of text.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_parse(p: i64, code: i64, str_code: i64) -> i64 {
    let raw = str_of(p);
    release(p);
    let (to, s) = (type_of(code), raw.trim());
    let big = to.is_ref() && shape(to) == &Shape::BigInt;
    let parsed = match to {
        Type::F64 => s.parse::<f64>().ok().map(|f| f.to_bits() as i64),
        Type::F32 => s.parse::<f32>().ok().map(crate::display::f32_slot),
        Type::Bool => match s {
            "true" => Some(1),
            "false" => Some(0),
            _ => None,
        },
        Type::Char => {
            let mut chars = raw.chars();
            chars
                .next()
                .filter(|_| chars.next().is_none())
                .map(|c| c as i64)
        }
        Type::Ref(_) if big => s.parse().ok().map(|n| crate::rt_bigint::new_big(n, code)),
        Type::Ref(_) => AnyDecimal::parse(s).map(|d| crate::rt_big_decimal::new_decimal(d, code)),
        _ => s
            .parse::<i128>()
            .ok()
            .filter(|n| (range(to).0..=range(to).1).contains(n))
            .map(|n| n as i64),
    };
    parsed.unwrap_or_else(|| {
        let name = match to {
            Type::F64 => "Float",
            Type::F32 => "Float32",
            Type::Ref(_) if big => "BigInt",
            Type::Ref(_) => "Decimal",
            t => t.kavach_name(),
        };
        let message = format!("`{raw}` is not a valid {name}");
        set_error(new_str(message, type_of(str_code)));
        0
    })
}
