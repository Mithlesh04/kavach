// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Printing values in native code. Printing follows spec 1.11 and must
//! match the interpreter (`compiler/interp/src/value.rs`). Equality,
//! ordering and hashing are in `equality.rs`.

use crate::heap::{Data, items_of, obj, shape, str_of, with_str};
use crate::rt::format_float;
use kavach_kir::{FieldShape, Shape, Type};
use kavach_runtime_core::Decimal;

/// The `Float32` in a slot: its 32 bits, zero-extended.
pub fn f32_of(slot: i64) -> f32 {
    f32::from_bits(slot as u32)
}

/// A `Float32` as a slot.
pub fn f32_slot(f: f32) -> i64 {
    i64::from(f.to_bits())
}

/// How `print` shows a value: text as is at the top, everything else (and
/// text inside collections) the way it is written in code.
pub fn display(slot: i64, ty: Type, top: bool) -> String {
    match ty {
        Type::F64 => format_float(f64::from_bits(slot as u64)),
        Type::F32 => kavach_runtime_core::format_float32(f32_of(slot)),
        Type::Bool => (slot != 0).to_string(),
        Type::Char => {
            let c = char::from_u32(slot as u32).unwrap_or('?');
            if top {
                c.to_string()
            } else {
                kavach_runtime_core::quoted(&c.to_string(), '\'')
            }
        }
        Type::Unit => "()".to_string(),
        Type::Ref(id) => {
            crate::rt_display::own_form(slot, id).unwrap_or_else(|| display_ref(slot, ty, top))
        }
        t if t.is_signed() => slot.to_string(),
        _ => (slot as u64).to_string(),
    }
}

/// Appends `display(slot, ty, true)` to `out`; numbers, booleans,
/// characters and text are written straight in, with no temporary text.
pub fn append_display(out: &mut crate::text::Text, slot: i64, ty: Type) {
    match ty {
        Type::Bool => out.push_str(if slot != 0 { "true" } else { "false" }),
        Type::Char => out.push_str(
            char::from_u32(slot as u32)
                .unwrap_or('?')
                .encode_utf8(&mut [0; 4]),
        ),
        Type::F64 | Type::F32 | Type::Unit => out.push_str(&display(slot, ty, true)),
        Type::Ref(_) if *shape(ty) == Shape::Str => with_str(slot, |t| out.push_str(t)),
        Type::Ref(_) => out.push_str(&display(slot, ty, true)),
        t if t.is_signed() => {
            if slot < 0 {
                out.push_str("-");
            }
            push_digits(out, slot.unsigned_abs());
        }
        _ => push_digits(out, slot as u64),
    }
}

/// Appends the decimal digits of `n`, written backwards into a small
/// buffer (faster than the general formatter, which `"{i}"` calls a lot).
fn push_digits(out: &mut crate::text::Text, n: u64) {
    out.push_str(int_text(&mut [0; 21], n, false));
}

/// Whole number `n` (its magnitude, with `negative` its sign) as text,
/// written backwards into `buf`.
pub(crate) fn int_text(buf: &mut [u8; 21], mut n: u64, negative: bool) -> &str {
    let mut at = buf.len();
    loop {
        at -= 1;
        buf[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if negative {
        at -= 1;
        buf[at] = b'-';
    }
    // SAFETY: the buffer holds only the ASCII digits and sign written above.
    #[allow(unsafe_code)]
    unsafe {
        std::str::from_utf8_unchecked(&buf[at..])
    }
}

/// How `print` and `main`'s error report show an error (spec 5.5).
pub fn show_error(p: i64) -> String {
    display(p, obj(p).ty(), true)
}

pub fn decimal_of(p: i64) -> Decimal {
    match &*crate::heap::data(p).borrow() {
        Data::Decimal(d) => **d,
        _ => Decimal::from_int(0),
    }
}

/// The scalar inside a boxed optional.
pub fn boxed(p: i64) -> i64 {
    match &*crate::heap::data(p).borrow() {
        Data::Boxed(v) => *v as i64,
        _ => 0,
    }
}

fn display_ref(p: i64, ty: Type, top: bool) -> String {
    let join = |items: &[u64], types: &[Type]| {
        items
            .iter()
            .zip(types)
            .map(|(s, t)| display(*s as i64, *t, false))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if p != 0
        && let Data::Context(c) = &*crate::heap::data(p).borrow()
    {
        return format!("{}: {}", c.0, show_error(c.1 as i64));
    }
    match shape(ty) {
        // An interface value prints as its concrete value (spec 2.15).
        Shape::Iface { .. } => {
            let s = items_of(p);
            display(s[0] as i64, crate::rt::type_of(s[1] as i64), top)
        }
        Shape::Optional(_) if p == 0 => "none".to_string(),
        Shape::Optional(inner) if inner.is_ref() => display(p, *inner, top),
        Shape::Optional(inner) => display(boxed(p), *inner, top),
        Shape::Decimal => crate::rt_big_decimal::show(p),
        Shape::BigInt => crate::rt_bigint::show(p),
        Shape::Regex => format!("Regex({:?})", crate::opaque::describe(p)),
        // Any error: the object knows its own type.
        Shape::Error => display(p, obj(p).ty(), top),
        Shape::Str if top => str_of(p),
        Shape::Bytes => crate::rt_bytes::with_bytes(p, |b| {
            let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
            format!("Bytes({hex})")
        }),
        Shape::Str => kavach_runtime_core::quoted(&str_of(p), '"'),
        Shape::List(t) => {
            let items = items_of(p);
            format!("[{}]", join(&items, &vec![*t; items.len()]))
        }
        Shape::Tuple(ts) => format!("({})", join(&items_of(p), ts)),
        Shape::Map(k, v) => {
            let items = items_of(p);
            // As a map is written (spec 2.2, decision 0129).
            if items.is_empty() {
                return "{:}".to_string();
            }
            let pairs: Vec<String> = items
                .chunks(2)
                .map(|e| {
                    let key = display(e[0] as i64, *k, false);
                    format!("{key}: {}", display(e[1] as i64, *v, false))
                })
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        Shape::Shared(t) if crate::send::is_weak(p) => {
            let shown = crate::send::with_weak_value(p, |v| display(v as i64, *t, false));
            format!("Weak({})", shown.unwrap_or_else(|| "none".to_string()))
        }
        Shape::Shared(t) => {
            let shown = crate::send::with_shared_value(p, |v| display(v as i64, *t, false));
            format!("Shared({})", shown.unwrap_or_default())
        }
        Shape::Func(..) | Shape::Closure(_) => "<function>".to_string(),
        Shape::Task(_) => "Task".to_string(),
        Shape::Reader(_) => "Reader".to_string(),
        Shape::Socket => "Socket".to_string(),
        Shape::Channel(_) => "Channel".to_string(),
        Shape::Set(t) => {
            let items = items_of(p);
            let shown: Vec<String> = items
                .iter()
                .step_by(2)
                .map(|s| display(*s as i64, *t, false))
                .collect();
            format!("Set.of({})", shown.join(", "))
        }
        Shape::Record { name, fields, .. } => fields_text(name, fields, &items_of(p)),
        // A `Json` prints as compact JSON text (spec 2.12).
        Shape::Union { .. } if crate::rt_json::is_json(ty) => {
            kavach_runtime_core::json::write(&crate::rt_json::tree(p), 0)
        }
        Shape::Union { variants, .. } => {
            let variant = crate::heap::tag_of(p) as usize;
            fields_text(
                &variants[variant].name,
                &variants[variant].fields,
                &items_of(p),
            )
        }
    }
}

fn fields_text(name: &str, fields: &[FieldShape], items: &[u64]) -> String {
    if fields.is_empty() {
        return name.to_string();
    }
    let parts: Vec<String> = fields
        .iter()
        .zip(items)
        .map(|(f, s)| {
            if f.secret {
                format!("{}: <secret>", f.name)
            } else {
                format!("{}: {}", f.name, display(*s as i64, f.ty, false))
            }
        })
        .collect();
    format!("{name}({})", parts.join(", "))
}
