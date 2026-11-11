//! `std.json` (spec 2.12) in native code: JSON text to and from `Json`
//! values, with the parser and writer the interpreter uses too
//! (`kavach_runtime_core::json`), so both read and write exactly alike.
//!
//! `Json` and `JsonError` are ordinary unions from the standard library
//! module `std.json` (`stdlib/json/json.kav`): a value is an object with
//! its variant's tag and one slot, or none for `Null`.

use crate::heap::{alloc_zeroed, items_of, obj, release, slots, slots_mut, tag_of};
use crate::rt::type_of;
use crate::rt_errors::set_error;
use crate::shapes::{new_text, shape, with_str};
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::json::{self, Error, Limits, Tree};

/// Whether `ty` is the built-in `Json` union (not a program's own type
/// that happens to have the same name).
pub fn is_json(ty: Type) -> bool {
    matches!(shape(ty), Shape::Union { name, variants }
        if name == "Json" && variants.first().is_some_and(|v| v.name == "Null"))
}

/// `json.parse(text, limits)` (owned; `limits` is a list of the three
/// limits) gives a `Json` of type `code`, or 0 with the error set.
/// `json.of(value)` and `json.stringify(value, indent)` get the value in
/// a one-item tuple, whose type says how to encode it; they give a `Json`
/// or text of type `code`.
pub fn json_op(op: RtOp, a: i64, b: i64, code: i64, line: i32) -> i64 {
    if matches!(op, RtOp::JsonOf | RtOp::JsonStringify) {
        let item = match shape(obj(a).ty()) {
            Shape::Tuple(ts) => ts[0],
            _ => Type::Unit,
        };
        // The static check covers written types; this covers values
        // reached through generics, such as `std.grpc`'s (spec 2.11).
        if may_hold_secret(item, &mut Vec::new())
            && let Some((holder, field)) = find_secret(slots(a)[0] as i64, item)
        {
            crate::rt::panic_with(
                &format!(
                    "`{holder}.{field}` is `secret`, so it can't be encoded as JSON; use `reveal(…)` on a copy if you really mean to (spec 2.11)"
                ),
                line,
            );
        }
        if op == RtOp::JsonStringify {
            let indent = usize::try_from(b).unwrap_or(0);
            let text = crate::rt_json_write::text_of(slots(a)[0] as i64, item, indent);
            release(a);
            return crate::shapes::new_str(text, type_of(code));
        }
        let t = encode(slots(a)[0] as i64, item);
        release(a);
        return value(&t, type_of(code));
    }
    let d = Limits::default();
    let limit = |i: usize, default: usize| {
        slots(b)
            .get(i)
            .map_or(default, |n| usize::try_from(*n as i64).unwrap_or(0))
    };
    let limits = Limits {
        max_depth: limit(0, d.max_depth),
        max_bytes: limit(1, d.max_bytes),
        max_fields: limit(2, d.max_fields),
        comments: limit(3, 0) != 0,
    };
    // Onto a tape (decision 0032), with no tree in between.
    let parsed = with_str(a, |s| {
        let mut build = crate::rt_json_build::Builder::new(Some(s));
        json::parse_with(s, &limits, &mut build).map(|root| build.finish(s.into(), root))
    });
    release(a);
    release(b);
    match parsed {
        Ok((doc, root)) => crate::rt_json_tape::value(&doc, root, type_of(code)),
        Err(e) => {
            set_error(error(e));
            0
        }
    }
}

/// Variant `name` of union `ty`, and the type of its first field.
fn variant(ty: Type, name: &str) -> (u32, Option<Type>) {
    let Shape::Union { variants, .. } = shape(ty) else {
        return (0, None);
    };
    let tag = variants.iter().position(|v| v.name == name).unwrap_or(0);
    (tag as u32, variants[tag].fields.first().map(|f| f.ty))
}

/// A new object of union `ty`: variant `name` with these field slots.
pub(crate) fn new_variant(ty: Type, name: &str, fields: &[u64]) -> i64 {
    let p = alloc_zeroed(ty, variant(ty, name).0, fields.len());
    slots_mut(p).copy_from_slice(fields);
    p
}

/// `t` as a `Json` value of type `ty`.
pub(crate) fn value(t: &Tree, ty: Type) -> i64 {
    let field = |name: &str| variant(ty, name).1.unwrap_or(Type::I64);
    let (name, slot) = match t {
        Tree::Null => return new_variant(ty, "Null", &[]),
        Tree::Bool(b) => ("Boolean", *b as u64),
        // Every number by its digits (decision 0071).
        Tree::Int(_) | Tree::Float(_) | Tree::Number(_) => match json::digits(t) {
            Some(d) => ("Number", new_text(&d, field("Number")) as u64),
            None => return new_variant(ty, "Null", &[]),
        },
        Tree::Text(s) => ("Text", new_text(s, field("Text")) as u64),
        Tree::Array(items) => {
            let list = alloc_zeroed(field("Array"), 0, items.len());
            for (slot, item) in slots_mut(list).iter_mut().zip(items) {
                *slot = value(item, ty) as u64;
            }
            ("Array", list as u64)
        }
        Tree::Object(fields) => {
            let map_ty = field("Object");
            let key_ty = match shape(map_ty) {
                Shape::Map(k, _) => *k,
                _ => Type::I64,
            };
            let mut map = crate::rt_maps::kv_map_new(crate::rt::type_code(map_ty));
            for (k, v) in fields {
                map = crate::rt_maps::kv_map_insert(map, new_text(k, key_ty), value(v, ty));
            }
            ("Object", map as u64)
        }
    };
    new_variant(ty, name, &[slot])
}

/// Any value `slot` (borrowed) of type `ty` as JSON (spec 2.12): what
/// `json.of` gives. A `Json` stays as it is.
/// Whether a value of type `ty` can hold a `secret` field, seen from its
/// type alone: most can't, and then the value needn't be walked.
fn may_hold_secret(ty: Type, seen: &mut Vec<Type>) -> bool {
    if !ty.is_ref() || seen.contains(&ty) {
        return false;
    }
    seen.push(ty);
    let any = |fields: &[kavach_kir::FieldShape], seen: &mut Vec<Type>| {
        fields
            .iter()
            .any(|f| f.secret || may_hold_secret(f.ty, seen))
    };
    match shape(ty) {
        Shape::Optional(t) | Shape::List(t) | Shape::Set(t) | Shape::Shared(t) => {
            may_hold_secret(*t, seen)
        }
        Shape::Map(k, v) => may_hold_secret(*k, seen) || may_hold_secret(*v, seen),
        Shape::Tuple(ts) => ts.iter().any(|t| may_hold_secret(*t, seen)),
        Shape::Record { fields, .. } => any(fields, seen),
        Shape::Union { variants, .. } => variants.iter().any(|v| any(&v.fields, seen)),
        _ => false,
    }
}

/// A `secret` field anywhere inside the value: who holds it and its name.
fn find_secret(slot: i64, ty: Type) -> Option<(String, String)> {
    if !ty.is_ref() || slot == 0 {
        return None;
    }
    let p = slot;
    let in_fields = |name: &str, fields: &[kavach_kir::FieldShape], items: &[u64]| {
        fields.iter().zip(items).find_map(|(f, s)| {
            if f.secret {
                Some((name.to_string(), f.name.clone()))
            } else {
                find_secret(*s as i64, f.ty)
            }
        })
    };
    match shape(ty) {
        Shape::Optional(inner) if inner.is_ref() => find_secret(p, *inner),
        Shape::Optional(inner) => find_secret(crate::display::boxed(p), *inner),
        Shape::Record { name, fields, .. } => in_fields(name, fields, &items_of(p)),
        Shape::Union { variants, .. } => {
            let v = &variants[crate::heap::tag_of(p) as usize];
            in_fields(&v.name, &v.fields, &items_of(p))
        }
        Shape::List(t) | Shape::Set(t) => {
            items_of(p).iter().find_map(|s| find_secret(*s as i64, *t))
        }
        Shape::Map(k, v) => items_of(p)
            .chunks(2)
            .find_map(|c| find_secret(c[0] as i64, *k).or_else(|| find_secret(c[1] as i64, *v))),
        Shape::Tuple(ts) => items_of(p)
            .iter()
            .zip(ts.iter())
            .find_map(|(s, t)| find_secret(*s as i64, *t)),
        Shape::Shared(_) if crate::send::is_weak(p) => None,
        Shape::Shared(t) => find_secret(slots(p)[0] as i64, *t),
        _ => None,
    }
}

pub(crate) fn encode(slot: i64, ty: Type) -> Tree {
    let object = |names: Vec<&str>, types: Vec<Type>, p: i64| {
        let items = items_of(p);
        Tree::Object(
            names
                .iter()
                .zip(types.iter().zip(items.iter()))
                .map(|(n, (t, s))| (n.to_string(), encode(*s as i64, *t)))
                .collect(),
        )
    };
    let all = |p: i64, t: Type, step: usize| {
        let items = items_of(p);
        Tree::Array(
            items
                .iter()
                .step_by(step)
                .map(|s| encode(*s as i64, t))
                .collect(),
        )
    };
    match ty {
        Type::F64 => return Tree::Float(f64::from_bits(slot as u64)),
        Type::F32 => return Tree::Float(f64::from(crate::display::f32_of(slot))),
        Type::Bool => return Tree::Bool(slot != 0),
        Type::Char => return Tree::Text(char::from_u32(slot as u32).unwrap_or('?').to_string()),
        Type::Unit => return Tree::Null,
        Type::U64 if slot < 0 => return Tree::Number((slot as u64).to_string()),
        t if !t.is_ref() => return Tree::Int(slot),
        _ => {}
    }
    let p = slot;
    match shape(ty) {
        Shape::Optional(_) if p == 0 => Tree::Null,
        Shape::Optional(inner) if inner.is_ref() => encode(p, *inner),
        Shape::Optional(inner) => encode(crate::display::boxed(p), *inner),
        Shape::Str => Tree::Text(with_str(p, str::to_string)),
        // With all their digits (decision 0071).
        Shape::Decimal => Tree::Number(match crate::rt_big_decimal::any_decimal_of(p) {
            kavach_runtime_core::AnyDecimal::Small(d) => d.to_string(),
            kavach_runtime_core::AnyDecimal::Big(b) => b.to_string(),
        }),
        Shape::BigInt => {
            crate::rt_bigint::big_of(p).map_or(Tree::Null, |n| Tree::Number(n.to_string()))
        }
        Shape::List(t) => all(p, *t, 1),
        Shape::Set(t) => all(p, *t, 2),
        Shape::Tuple(ts) => {
            let items = items_of(p);
            Tree::Array(
                ts.iter()
                    .zip(items.iter())
                    .map(|(t, s)| encode(*s as i64, *t))
                    .collect(),
            )
        }
        Shape::Map(_, v) => {
            let items = items_of(p);
            Tree::Object(
                items
                    .chunks(2)
                    .map(|e| {
                        (
                            with_str(e[0] as i64, str::to_string),
                            encode(e[1] as i64, *v),
                        )
                    })
                    .collect(),
            )
        }
        Shape::Record { fields, .. } => object(
            fields.iter().map(|f| f.name.as_str()).collect(),
            fields.iter().map(|f| f.ty).collect(),
            p,
        ),
        Shape::Union { .. } if is_json(ty) => tree(p),
        Shape::Union { variants, .. } => {
            let v = &variants[tag_of(crate::shapes::innermost(p)) as usize];
            if v.fields.is_empty() {
                return Tree::Text(v.name.clone());
            }
            let fields = object(
                v.fields.iter().map(|f| f.name.as_str()).collect(),
                v.fields.iter().map(|f| f.ty).collect(),
                crate::shapes::innermost(p),
            );
            Tree::Object(vec![(v.name.clone(), fields)])
        }
        _ => Tree::Null,
    }
}

/// The `Json` value `p` (borrowed) as a tree for the writer.
pub fn tree(p: i64) -> Tree {
    if let Some((doc, w)) = crate::rt_json_tape::on_tape(p) {
        return doc.tree(w);
    }
    let ty = obj(p).ty();
    let Shape::Union { variants, .. } = shape(ty) else {
        return Tree::Null;
    };
    let slot = slots(p).first().copied().unwrap_or(0);
    match variants[tag_of(p) as usize].name.as_str() {
        "Boolean" => Tree::Bool(slot != 0),
        // Digits that aren't a JSON number are written `null`, as `NaN` is.
        "Number" => with_str(slot as i64, |d| match json::is_number(d) {
            true => Tree::Number(d.to_string()),
            false => Tree::Null,
        }),
        "Text" => Tree::Text(with_str(slot as i64, str::to_string)),
        "Array" => Tree::Array(slots(slot as i64).iter().map(|i| tree(*i as i64)).collect()),
        "Object" => Tree::Object(
            items_of(slot as i64)
                .chunks(2)
                .map(|e| (with_str(e[0] as i64, str::to_string), tree(e[1] as i64)))
                .collect(),
        ),
        _ => Tree::Null,
    }
}

/// A `JsonError.Mismatch`: decoding wanted `want` at `path`.
pub(crate) fn mismatch(path: &str, want: &str) -> i64 {
    let Some(ty) = crate::shapes::find_union("JsonError", "Syntax") else {
        return 0;
    };
    let text_ty = match shape(ty) {
        Shape::Union { variants, .. } => variants
            .iter()
            .flat_map(|v| &v.fields)
            .find(|f| f.name == "path")
            .map_or(Type::I64, |f| f.ty),
        _ => Type::I64,
    };
    let fields = [
        new_text(path, text_ty) as u64,
        new_text(want, text_ty) as u64,
    ];
    new_variant(ty, "Mismatch", &fields)
}

/// `e` as a `JsonError` value.
pub(crate) fn error(e: Error) -> i64 {
    let Some(ty) = crate::shapes::find_union("JsonError", "Syntax") else {
        return 0;
    };
    let text = |name: &str, s: &str| {
        let Shape::Union { variants, .. } = shape(ty) else {
            return 0;
        };
        let field = variants
            .iter()
            .flat_map(|v| &v.fields)
            .find(|f| f.name == name)
            .map_or(Type::I64, |f| f.ty);
        new_text(s, field) as u64
    };
    match e {
        Error::Syntax {
            line,
            column,
            message,
        } => new_variant(
            ty,
            "Syntax",
            &[line as u64, column as u64, text("message", &message)],
        ),
        Error::TooDeep(limit) => new_variant(ty, "TooDeep", &[limit as u64]),
        Error::TooLarge(limit) => new_variant(ty, "TooLarge", &[limit as u64]),
        Error::DuplicateKey { key, line, column } => new_variant(
            ty,
            "DuplicateKey",
            &[text("key", &key), line as u64, column as u64],
        ),
    }
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_json, |op, a, b, c, code, line| {
    match op {
        RtOp::JsonDecode | RtOp::JsonDecodeValue => {
            crate::rt_json_decode::decode_op(op, a, b, c, code)
        }
        RtOp::JsonParse | RtOp::JsonStringify | RtOp::JsonOf => json_op(op, a, b, code, line),
        _ => crate::rt_json_read::read_op(op, a, b, code),
    }
});
