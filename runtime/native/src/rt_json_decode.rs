//! `json.decode<T>` and `json.decode_value<T>` (spec 2.12) in native code:
//! JSON into a value of type `T`, read from the program's shape table,
//! exactly as the interpreter does (`compiler/interp/src/json_decode.rs`).
//!
//! Field defaults are code, so generated code passes them in: a tuple of
//! *templates*, one record per type with defaults, holding them (other
//! fields are 0), each followed by an `Int` whose bit `i` says field `i`
//! has a default. A missing field is copied from its type's template.

use crate::heap::{
    Data, alloc, alloc_slots, alloc_zeroed, obj, release, release_slot, retain, slots, slots_mut,
};
use crate::rt::{type_code, type_of};
use crate::rt_errors::set_error;
use crate::shapes::{new_text, shape, with_str};
use kavach_kir::{FieldShape, RtOp, Shape, Type};
use kavach_runtime_core::json::{self, Limits, Tree, expected, path_field, path_item};
use kavach_runtime_core::{Decimal, format_float};

/// Where decoding failed, and what it wanted there.
type Failed = (String, String);

thread_local! {
    /// While `json.decode` decodes: the problems found so far. Decoding
    /// then goes on after one, with an empty slot in its place, so the
    /// error lists every problem (spec 2.12, decision 0086). Other callers
    /// stop at the first.
    static PROBLEMS: std::cell::RefCell<Option<Vec<Failed>>> =
        const { std::cell::RefCell::new(None) };
    /// Whether decoding skips fields its type doesn't have: `json.decode`
    /// sets it from `unknown` (decision 0096); other readers, such as an
    /// agent's saved state, skip them.
    pub(crate) static IGNORE: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// `e`, unless the problems are being collected: then it is noted, and
/// decoding goes on.
fn stop_at(e: Failed) -> Option<Failed> {
    PROBLEMS.with(|p| match p.borrow_mut().as_mut() {
        Some(all) => {
            all.push(e);
            None
        }
        None => Some(e),
    })
}

/// Decodes `t` as type `ty` for `json.decode`, listing every problem:
/// those of shape, or, when the shapes fit, each broken invariant.
fn decode_every(t: &Tree, ty: Type, templates: i64) -> Result<u64, Vec<Failed>> {
    PROBLEMS.with(|p| *p.borrow_mut() = Some(Vec::new()));
    let result = decode(t, ty, "", templates);
    let mut problems = PROBLEMS.with(|p| p.borrow_mut().take()).unwrap_or_default();
    let slot = match result {
        Ok(slot) => slot,
        Err(e) => {
            problems.push(e);
            0
        }
    };
    if problems.is_empty() {
        problems = crate::rt_json_checks::broken(slot, ty);
    }
    if problems.is_empty() {
        return Ok(slot);
    }
    release_slot(slot as i64, ty);
    Err(problems)
}

/// `json.decode<T>(text, limits, templates)` or
/// `json.decode_value<T>(json, 0, templates)`, all owned: a value of type
/// `code`, or 0 with the error set.
pub fn decode_op(op: RtOp, a: i64, b: i64, c: i64, code: i64) -> i64 {
    // The last of `b`'s numbers says whether unknown fields are skipped.
    IGNORE.with(|i| i.set(slots(b).last().is_some_and(|n| *n != 0)));
    let tree = if op == RtOp::JsonDecode {
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
            comments: false,
        };
        // The fast way first: straight from the text. Only when that
        // fails is the text parsed into a tree, to say why.
        let ty = type_of(code);
        if let Some(v) = with_str(a, |s| crate::rt_json_pull::decode(s, &limits, ty, c)) {
            let broken = crate::rt_json_checks::broken(v, ty);
            if broken.is_empty() {
                return finish(a, b, c, v as i64);
            }
            release_slot(v as i64, ty);
            set_error(crate::rt_json::invalid(&broken));
            return finish(a, b, c, 0);
        }
        let parsed = with_str(a, |s| json::parse(s, &limits));
        match parsed {
            Ok(t) => t,
            Err(e) => {
                set_error(crate::rt_json::error(e));
                return finish(a, b, c, 0);
            }
        }
    } else {
        crate::rt_json::tree(a)
    };
    let result = match decode_every(&tree, type_of(code), c) {
        Ok(slot) => slot as i64,
        Err(problems) => {
            set_error(crate::rt_json::invalid(&problems));
            0
        }
    };
    finish(a, b, c, result)
}

fn finish(a: i64, b: i64, c: i64, result: i64) -> i64 {
    IGNORE.with(|i| i.set(true));
    release(a);
    release(b);
    release(c);
    result
}

pub(crate) fn fits(n: i64, ty: Type) -> bool {
    match ty {
        Type::I8 => i8::try_from(n).is_ok(),
        Type::I16 => i16::try_from(n).is_ok(),
        Type::I32 => i32::try_from(n).is_ok(),
        Type::U8 => u8::try_from(n).is_ok(),
        Type::U16 => u16::try_from(n).is_ok(),
        Type::U32 => u32::try_from(n).is_ok(),
        Type::U64 => n >= 0,
        _ => true,
    }
}

/// `t` as an owned slot of type `ty`.
pub(crate) fn decode(t: &Tree, ty: Type, path: &str, templates: i64) -> Result<u64, Failed> {
    let wrong = |want: &str| Err((path.to_string(), want.to_string()));
    match (ty, t) {
        (Type::Bool, Tree::Bool(b)) => return Ok(*b as u64),
        (Type::Bool, _) => return wrong(expected::BOOL),
        (Type::F64, Tree::Int(n)) => return Ok((*n as f64).to_bits()),
        (Type::F64, Tree::Float(f)) => return Ok(f.to_bits()),
        (Type::F64, Tree::Number(text)) => {
            return float_of(text).map_or(wrong(&expected::float(ty.kavach_name())), |f| {
                Ok(f.to_bits())
            });
        }
        (Type::F64, _) => return wrong(expected::NUMBER),
        (Type::F32, Tree::Int(n)) => return Ok(crate::display::f32_slot(*n as f32) as u64),
        (Type::F32, Tree::Float(f)) => return Ok(crate::display::f32_slot(*f as f32) as u64),
        (Type::F32, Tree::Number(text)) => {
            return float_of(text).map_or(wrong(&expected::float(ty.kavach_name())), |f| {
                Ok(crate::display::f32_slot(f as f32) as u64)
            });
        }
        (Type::F32, _) => return wrong(expected::NUMBER),
        (Type::Char, Tree::Text(s)) if s.chars().count() == 1 => {
            return Ok(s.chars().next().unwrap_or(' ') as u64);
        }
        (Type::Char, _) => return wrong(expected::CHAR),
        (t, Tree::Int(n)) if t.is_int() && fits(*n, t) => return Ok(*n as u64),
        // A `Json` value's number, by its digits (`json.decode_value`).
        (t, Tree::Number(text)) if t.is_int() => {
            return match text.parse::<i64>() {
                Ok(n) if fits(n, t) => Ok(n as u64),
                _ => wrong(&expected::integer(t.kavach_name())),
            };
        }
        (t, _) if t.is_int() => return wrong(&expected::integer(t.kavach_name())),
        (Type::Unit, _) => return Ok(0),
        _ => {}
    }
    match (shape(ty), t) {
        (Shape::Str, Tree::Text(s)) => Ok(new_text(s, ty) as u64),
        (Shape::Str, _) => wrong(expected::TEXT),
        (Shape::Decimal, Tree::Int(n)) => Ok(decimal(Decimal::from_int(*n as i128), ty)),
        (Shape::Decimal, Tree::Float(f)) => {
            let d = Decimal::parse(&format_float(*f)).unwrap_or(Decimal::from_int(0));
            Ok(decimal(d, ty))
        }
        (Shape::Decimal | Shape::BigInt, Tree::Number(text)) => {
            exact_number(text, ty).ok_or_else(|| (path.to_string(), wanted(ty).to_string()))
        }
        (Shape::BigInt, Tree::Int(n)) => exact_number(&n.to_string(), ty)
            .ok_or_else(|| (path.to_string(), expected::WHOLE.to_string())),
        (Shape::Decimal, _) => wrong(expected::NUMBER),
        (Shape::BigInt, _) => wrong(expected::WHOLE),
        (Shape::Optional(_), Tree::Null) => Ok(0),
        (Shape::Optional(inner), _) => {
            let v = decode(t, *inner, path, templates)?;
            if inner.is_ref() {
                return Ok(v);
            }
            Ok(crate::rt_data::kv_some(v as i64, type_code(ty)) as u64)
        }
        (Shape::List(item), Tree::Array(items)) => {
            let built = decode_all(items, &vec![*item; items.len()], path, templates)?;
            Ok(alloc_slots(ty, 0, built) as u64)
        }
        (Shape::Set(item), Tree::Array(items)) => {
            let built = decode_all(items, &vec![*item; items.len()], path, templates)?;
            let mut set = crate::rt_maps::kv_map_new(type_code(ty));
            for v in built {
                set = crate::rt_maps::kv_map_insert(set, v as i64, 0);
            }
            Ok(set as u64)
        }
        (Shape::Tuple(ts), Tree::Array(items)) if ts.len() == items.len() => {
            let built = decode_all(items, ts, path, templates)?;
            Ok(fixed(ty, 0, &built))
        }
        (Shape::Tuple(ts), _) => wrong(&expected::items(ts.len())),
        (Shape::List(_) | Shape::Set(_), _) => wrong(expected::ARRAY),
        (Shape::Map(k, v), Tree::Object(fields)) => {
            let types = vec![*v; fields.len()];
            let values: Vec<Tree> = fields.iter().map(|(_, v)| v.clone()).collect();
            let keys: Vec<String> = fields.iter().map(|(k, _)| path_field(path, k)).collect();
            let mut built = Vec::with_capacity(values.len());
            for (i, (value, at)) in values.iter().zip(&keys).enumerate() {
                match decode(value, types[i], at, templates).map_err(stop_at) {
                    Ok(slot) => built.push(slot),
                    Err(None) => built.push(0),
                    Err(Some(e)) => return Err(drop_all(built, &types, e)),
                }
            }
            let mut map = crate::rt_maps::kv_map_new(type_code(ty));
            for ((key, _), value) in fields.iter().zip(built) {
                map = crate::rt_maps::kv_map_insert(map, new_text(key, *k), value as i64);
            }
            Ok(map as u64)
        }
        (Shape::Map(..), _) => wrong(expected::OBJECT),
        (Shape::Union { .. }, _) if crate::rt_json::is_json(ty) => {
            Ok(crate::rt_json::value(t, ty) as u64)
        }
        (Shape::Record { fields, .. }, Tree::Object(given)) => {
            let built = decode_fields(given, fields, ty, path, templates)?;
            Ok(fixed(ty, 0, &built))
        }
        (Shape::Union { name, variants }, _) => {
            let found = match t {
                Tree::Text(v) => variants
                    .iter()
                    .position(|x| x.name == *v && x.fields.is_empty())
                    .map(|i| (i, Vec::new())),
                Tree::Object(one) if one.len() == 1 => {
                    match (&one[0].1, variants.iter().position(|x| x.name == one[0].0)) {
                        (Tree::Object(given), Some(i)) => Some((i, given.clone())),
                        _ => None,
                    }
                }
                _ => None,
            };
            let Some((i, given)) = found else {
                return wrong(&expected::variant(name));
            };
            let at = path_field(path, &variants[i].name);
            let built = decode_fields(&given, &variants[i].fields, Type::Unit, &at, templates)?;
            Ok(fixed(ty, i as u32, &built))
        }
        _ => wrong(expected::OBJECT),
    }
}

pub(crate) fn decimal(d: Decimal, ty: Type) -> u64 {
    alloc(ty, Data::Decimal(Box::new(d))) as u64
}

/// The number written as `text` as a `Decimal` or `BigInt` of type `ty`,
/// exactly (decision 0071): `None` when it isn't one, or its exponent is
/// past `EXPONENT_LIMIT`.
pub(crate) fn exact_number(text: &str, ty: Type) -> Option<u64> {
    let code = crate::rt::type_code(ty);
    match shape(ty) {
        Shape::Decimal if json::exponent_fits(text) => {
            let d = kavach_runtime_core::AnyDecimal::parse(text)?;
            Some(crate::rt_big_decimal::new_decimal(d, code) as u64)
        }
        Shape::BigInt => Some(crate::rt_bigint::new_big(text.parse().ok()?, code) as u64),
        _ => None,
    }
}

/// What a `Decimal` or `BigInt` field wants, for a `JsonProblem`.
fn wanted(ty: Type) -> &'static str {
    match shape(ty) {
        Shape::BigInt => expected::WHOLE,
        _ => expected::NUMBER,
    }
}

/// The number written as `text` as a `Float`: `None` past its range.
pub(crate) fn float_of(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|f| f.is_finite())
}

/// A new tuple, record or variant with these field slots (owned).
pub(crate) fn fixed(ty: Type, tag: u32, fields: &[u64]) -> u64 {
    let p = alloc_zeroed(ty, tag, fields.len());
    slots_mut(p).copy_from_slice(fields);
    p as u64
}

/// Each item decoded with its type; on a failure, the ones made so far
/// are released.
fn decode_all(
    items: &[Tree],
    types: &[Type],
    path: &str,
    templates: i64,
) -> Result<Vec<u64>, Failed> {
    let mut built = Vec::with_capacity(items.len());
    for (i, (item, t)) in items.iter().zip(types).enumerate() {
        match decode(item, *t, &path_item(path, i), templates).map_err(stop_at) {
            Ok(slot) => built.push(slot),
            Err(None) => built.push(0),
            Err(Some(e)) => return Err(drop_all(built, types, e)),
        }
    }
    Ok(built)
}

fn drop_all(built: Vec<u64>, types: &[Type], e: Failed) -> Failed {
    for (slot, t) in built.into_iter().zip(types) {
        release_slot(slot as i64, *t);
    }
    e
}

/// The fields of a record (of type `record`, which may have a template)
/// or variant: each from `given`, or its default, or `none`.
fn decode_fields(
    given: &[(String, Tree)],
    fields: &[FieldShape],
    record: Type,
    path: &str,
    templates: i64,
) -> Result<Vec<u64>, Failed> {
    let types: Vec<Type> = fields.iter().map(|f| f.ty).collect();
    // A field the type doesn't have (decision 0096).
    if !IGNORE.with(std::cell::Cell::get) {
        for (k, _) in given {
            if !fields.iter().any(|f| f.name == *k)
                && let Some(e) = stop_at((path_field(path, k), expected::NO_FIELD.to_string()))
            {
                return Err(e);
            }
        }
    }
    let mut built = Vec::with_capacity(fields.len());
    for (i, f) in fields.iter().enumerate() {
        let slot = match given.iter().find(|(k, _)| *k == f.name) {
            Some((_, t)) => decode(t, f.ty, &path_field(path, &f.name), templates),
            None => match template(templates, record) {
                Some((p, defaults)) if i < 64 && defaults >> i & 1 == 1 => {
                    let slot = slots(p)[i];
                    if f.ty.is_ref() {
                        retain(slot as i64);
                    }
                    Ok(slot)
                }
                _ if matches!(shape(f.ty), Shape::Optional(_)) && f.ty.is_ref() => Ok(0),
                _ => Err((path.to_string(), expected::field(&f.name))),
            },
        };
        match slot.map_err(stop_at) {
            Ok(s) => built.push(s),
            Err(None) => built.push(0),
            Err(Some(e)) => return Err(drop_all(built, &types, e)),
        }
    }
    Ok(built)
}

/// The template record of type `record` and its mask of fields with
/// defaults, if generated code passed one.
pub(crate) fn template(templates: i64, record: Type) -> Option<(i64, u64)> {
    if templates == 0 || record == Type::Unit {
        return None;
    }
    slots(templates)
        .chunks(2)
        .find(|pair| pair[0] != 0 && obj(pair[0] as i64).ty() == record)
        .map(|pair| (pair[0] as i64, pair.get(1).copied().unwrap_or(0)))
}
