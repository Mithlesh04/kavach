//! Printing and comparing values in native code. Printing follows spec
//! 1.11 and must match the interpreter (`compiler/interp/src/value.rs`).

use crate::hashing::FastHasher;
use crate::heap::{Data, items_of, obj, shape, str_of, with_str};
use crate::rt::format_float;
use kavach_kir::{FieldShape, Shape, Type};
use kavach_runtime_core::Decimal;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

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
            if top { c.to_string() } else { format!("{c:?}") }
        }
        Type::Unit => "()".to_string(),
        Type::Ref(_) => display_ref(slot, ty, top),
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
        Shape::Str => format!("{:?}", str_of(p)),
        Shape::List(t) => {
            let items = items_of(p);
            format!("[{}]", join(&items, &vec![*t; items.len()]))
        }
        Shape::Tuple(ts) => format!("({})", join(&items_of(p), ts)),
        Shape::Map(k, v) => {
            let items = items_of(p);
            if items.is_empty() {
                return "[:]".to_string();
            }
            let pairs: Vec<String> = items
                .chunks(2)
                .map(|e| {
                    let key = display(e[0] as i64, *k, false);
                    format!("{key}: {}", display(e[1] as i64, *v, false))
                })
                .collect();
            format!("[{}]", pairs.join(", "))
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

/// Structural equality (spec 1.8), without consuming either value.
pub fn equal(a: i64, b: i64, ty: Type) -> bool {
    match ty {
        Type::F64 => f64::from_bits(a as u64) == f64::from_bits(b as u64),
        Type::F32 => f32_of(a) == f32_of(b),
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

/// Ordering of text, decimals and tuples (spec 1.8).
pub fn compare(a: i64, b: i64) -> Ordering {
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
        // 0.0 and -0.0 are equal, so they hash the same.
        Type::F64 => (f64::from_bits(slot as u64) + 0.0).to_bits().hash(h),
        Type::F32 => (f32_of(slot) + 0.0).to_bits().hash(h),
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
