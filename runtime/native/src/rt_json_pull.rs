// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.decode<T>(text)` straight from the text (spec 2.12), with no
//! `Json` or tree in between: the decoder asks the reader
//! (`kavach_runtime_core::json::Pull`) for what type `T` expects next.
//! This is the fast path only. Anything wrong, in the text or in its
//! shape, gives `None`, and decoding runs again through the tree
//! (`rt_json_decode.rs`), which reports the error exactly as before.

use crate::heap::{alloc_slots, release, release_slot, retain, slots};
use crate::rt::type_code;
use crate::rt_json_decode::{decimal, fixed, int_fits, template};
use crate::rt_json_wholes::part;
use crate::shapes::{new_text, shape};
use kavach_kir::{FieldShape, Shape, Type};
use kavach_runtime_core::Decimal;
use kavach_runtime_core::json::{Keys, Limits, Pull};

/// The value of type `ty` written in `text`, as an owned slot, or `None`
/// when the text is not one.
pub fn decode(text: &str, limits: &Limits, ty: Type, templates: i64) -> Option<u64> {
    let mut r = Pull::new(text, limits).ok()?;
    let v = read(&mut r, ty, templates, crate::rt_json_wholes::root()).ok()?;
    if r.end().is_err() {
        release_slot(v as i64, ty);
        return None;
    }
    Some(v)
}

/// Decoding failed; what it made so far is released.
struct Wrong;

type Read = Result<u64, Wrong>;

/// With node `node` of the plan of whole numbers with their own limits
/// (`rt_json_wholes.rs`).
fn read(r: &mut Pull<'_>, ty: Type, templates: i64, node: usize) -> Read {
    // A number's text; each type below reads it as it needs, with no
    // `Tree` in between.
    fn number<'t>(r: &mut Pull<'t>) -> Result<(&'t str, bool), Wrong> {
        match r.peek() {
            Some(b'-' | b'0'..=b'9') => r.number_text().map_err(|_| Wrong),
            _ => Err(Wrong),
        }
    }
    let float = |text: &str| crate::rt_json_decode::float_of(text).ok_or(Wrong);
    match ty {
        Type::Bool if matches!(r.peek(), Some(b't' | b'f')) => {
            return r.bool().map(u64::from).map_err(|_| Wrong);
        }
        Type::Bool => return Err(Wrong),
        Type::F32 => {
            let f = float(number(r)?.0)?;
            return Ok(crate::display::f32_slot(f as f32) as u64);
        }
        Type::F64 => return Ok(float(number(r)?.0)?.to_bits()),
        Type::Char => {
            let s = r.text().map_err(|_| Wrong)?;
            let mut chars = s.chars();
            return match (chars.next(), chars.next()) {
                (Some(c), None) => Ok(c as u64),
                _ => Err(Wrong),
            };
        }
        t if t.is_int() => {
            return match number(r)? {
                (text, true) => match text.parse::<i64>() {
                    Ok(n) if int_fits(n, t, node) => Ok(n as u64),
                    _ => Err(Wrong),
                },
                _ => Err(Wrong),
            };
        }
        Type::Unit => return r.skip().map(|_| 0).map_err(|_| Wrong),
        _ => {}
    }
    match shape(ty) {
        Shape::Str => {
            let s = r.text().map_err(|_| Wrong)?;
            Ok(new_text(&s, ty) as u64)
        }
        // A `bits` type wider than 64: a `BigInt` within its width.
        Shape::BigInt if crate::rt_json_wholes::wide(node).is_some() => {
            let w = crate::rt_json_wholes::wide(node).ok_or(Wrong)?;
            match number(r)? {
                (text, true) => match text.parse::<kavach_runtime_core::BigInt>() {
                    Ok(n) if w.fits(&n) => Ok(crate::rt_bigint::new_big(n, type_code(ty)) as u64),
                    _ => Err(Wrong),
                },
                _ => Err(Wrong),
            }
        }
        // Written as text, as `big_as_text` writes it (decision 0132).
        Shape::Decimal | Shape::BigInt if r.peek() == Some(b'"') => {
            let text = r.text().map_err(|_| Wrong)?;
            match kavach_runtime_core::json::is_number(&text) {
                true => crate::rt_json_decode::exact_number(&text, ty).ok_or(Wrong),
                false => Err(Wrong),
            }
        }
        // Exactly, from the digits (decision 0071); the 128-bit form first.
        Shape::Decimal => {
            let (text, _) = number(r)?;
            match Decimal::parse(text) {
                Some(d) => Ok(decimal(d, ty)),
                None => crate::rt_json_decode::exact_number(text, ty).ok_or(Wrong),
            }
        }
        Shape::BigInt => match number(r)? {
            (text, true) => crate::rt_json_decode::exact_number(text, ty).ok_or(Wrong),
            _ => Err(Wrong),
        },
        Shape::Optional(_) if r.peek() == Some(b'n') => r.null().map(|_| 0).map_err(|_| Wrong),
        Shape::Optional(inner) => {
            let v = read(r, *inner, templates, part(node, 0))?;
            if inner.is_ref() {
                return Ok(v);
            }
            Ok(crate::rt_data::kv_some(v as i64, type_code(ty)) as u64)
        }
        Shape::List(item) => {
            let built = items(r, |_| (*item, part(node, 0)), templates)?;
            Ok(alloc_slots(ty, 0, built) as u64)
        }
        Shape::Set(item) => {
            let built = items(r, |_| (*item, part(node, 0)), templates)?;
            let mut set = crate::rt_maps::kv_map_new(type_code(ty));
            for v in built {
                set = crate::rt_maps::kv_map_insert(set, v as i64, 0, 0);
            }
            Ok(set as u64)
        }
        Shape::Tuple(ts) => {
            let n = ts.len();
            let at = |i| (ts.get(i).copied().unwrap_or(Type::Unit), part(node, i));
            let built = items(r, at, templates)?;
            if built.len() != n {
                release_all(built, ts);
                return Err(Wrong);
            }
            Ok(fixed(ty, 0, &built))
        }
        Shape::Map(k, v) => map(r, ty, (*k, *v), templates, node),
        Shape::Union { .. } if crate::rt_json::is_json(ty) => {
            let mut b = crate::rt_json_build::Builder::new(None);
            let root = r.value(&mut b).map_err(|_| Wrong)?;
            let (doc, root) = b.finish("".into(), root);
            Ok(crate::rt_json_tape::value(&doc, root, ty) as u64)
        }
        Shape::Record { .. } if crate::rt_json::date_kind(ty).is_some() => {
            let kind = crate::rt_json::date_kind(ty).ok_or(Wrong)?;
            let text = r.text().map_err(|_| Wrong)?;
            let n = kavach_runtime_core::json::dates::read(kind, &text).ok_or(Wrong)?;
            Ok(crate::rt_json::date_value(ty, kind, &n))
        }
        Shape::Record { fields, .. } => {
            r.object().map_err(|_| Wrong)?;
            let nodes = crate::rt_json_wholes::fields(node, fields.len());
            let built = fields_of(r, fields, ty, templates, &nodes)?;
            Ok(fixed(ty, 0, &built))
        }
        // A name, or the text or number a variant carries (spec 2.6).
        Shape::Union { variants, .. } if r.peek() != Some(b'{') => {
            let t = match r.peek() {
                Some(b'"') => {
                    kavach_runtime_core::json::Tree::Text(r.text().map_err(|_| Wrong)?.into_owned())
                }
                _ => r.number().map_err(|_| Wrong)?,
            };
            match crate::rt_json::plain_variant_at(variants, &t) {
                Some(i) => Ok(fixed(ty, i as u32, &[])),
                None => Err(Wrong),
            }
        }
        // `{"Variant": {fields}}`: an object with one field.
        Shape::Union { variants, .. } => {
            let mut keys = Keys::default();
            r.object().map_err(|_| Wrong)?;
            let name = r.field(&mut keys).map_err(|_| Wrong)?.ok_or(Wrong)?;
            let i = variants.iter().position(|x| x.name == *name).ok_or(Wrong)?;
            r.object().map_err(|_| Wrong)?;
            let nodes = crate::rt_json_wholes::variant(node, i);
            let built = fields_of(r, &variants[i].fields, Type::Unit, templates, &nodes)?;
            if !matches!(r.field(&mut keys), Ok(None)) {
                let types: Vec<Type> = variants[i].fields.iter().map(|f| f.ty).collect();
                release_all(built, &types);
                return Err(Wrong);
            }
            Ok(fixed(ty, i as u32, &built))
        }
        _ => Err(Wrong),
    }
}

/// The items of an array, item `i` of type and plan node `item(i)`.
fn items(
    r: &mut Pull<'_>,
    item: impl Fn(usize) -> (Type, usize),
    templates: i64,
) -> Result<Vec<u64>, Wrong> {
    r.array().map_err(|_| Wrong)?;
    let mut built = Vec::new();
    let mut types = Vec::new();
    loop {
        match r.item(built.is_empty()) {
            Ok(true) => {}
            Ok(false) => return Ok(built),
            Err(_) => return Err(release_all(built, &types)),
        }
        let (t, node) = item(built.len());
        match read(r, t, templates, node) {
            Ok(v) => {
                built.push(v);
                types.push(t);
            }
            Err(e) => {
                release_all(built, &types);
                return Err(e);
            }
        }
    }
}

fn map(r: &mut Pull<'_>, ty: Type, (k, v): (Type, Type), templates: i64, node: usize) -> Read {
    r.object().map_err(|_| Wrong)?;
    let mut keys = Keys::default();
    let mut map = crate::rt_maps::kv_map_new(type_code(ty));
    let text_keys = k.is_ref() && matches!(shape(k), Shape::Str);
    loop {
        let key = match r.field(&mut keys) {
            Ok(Some(key)) => key,
            Ok(None) => return Ok(map as u64),
            Err(_) => break,
        };
        // A key is text, read back as the key type (decision 0132).
        let key = match text_keys {
            true => new_text(&key, k),
            false => {
                let tree =
                    kavach_runtime_core::json::key_tree(&key, crate::rt_json_decode::key_kind(k));
                match crate::rt_json_decode::decode_at(&tree, k, "", templates, part(node, 0)) {
                    Ok(slot) => slot as i64,
                    Err(_) => break,
                }
            }
        };
        let Ok(value) = read(r, v, templates, part(node, 1)) else {
            crate::rt_json_decode::release_key(key, k);
            break;
        };
        map = crate::rt_maps::kv_map_insert(map, key, value as i64, 0);
    }
    release(map);
    Err(Wrong)
}

/// The fields of a record (of type `record`, which may have a template)
/// or variant, after its `{`: each from the text, or its default, or
/// `none`.
fn fields_of(
    r: &mut Pull<'_>,
    fields: &[FieldShape],
    record: Type,
    templates: i64,
    nodes: &[usize],
) -> Result<Vec<u64>, Wrong> {
    let types: Vec<Type> = fields.iter().map(|f| f.ty).collect();
    let mut given: Vec<Option<u64>> = vec![None; fields.len()];
    let mut keys = Keys::default();
    let failed = loop {
        let name = match r.field(&mut keys) {
            Ok(Some(name)) => name,
            Ok(None) => break false,
            Err(_) => break true,
        };
        let Some(i) = fields.iter().position(|f| f.name == *name) else {
            // Refused unknown fields are reported by the slower decoder.
            let ignore = crate::rt_json_decode::IGNORE.with(std::cell::Cell::get);
            if !ignore || r.skip().is_err() {
                break true;
            }
            continue;
        };
        match read(
            r,
            fields[i].ty,
            templates,
            nodes.get(i).copied().unwrap_or(0),
        ) {
            Ok(v) => given[i] = Some(v),
            Err(_) => break true,
        }
    };
    let release_given = |given: &[Option<u64>]| {
        for (v, t) in given.iter().zip(&types) {
            if let Some(v) = v {
                release_slot(*v as i64, *t);
            }
        }
    };
    if failed {
        release_given(&given);
        return Err(Wrong);
    }
    let mut built = Vec::with_capacity(fields.len());
    for (i, f) in fields.iter().enumerate() {
        let slot = match given[i] {
            Some(v) => v,
            None => match template(templates, record) {
                Some((p, defaults)) if i < 64 && defaults >> i & 1 == 1 => {
                    let slot = slots(p)[i];
                    if f.ty.is_ref() {
                        retain(slot as i64);
                    }
                    slot
                }
                _ if matches!(shape(f.ty), Shape::Optional(_)) && f.ty.is_ref() => 0,
                _ => {
                    // A required field is missing.
                    for (v, t) in given.iter().zip(&types).skip(i) {
                        if let Some(v) = v {
                            release_slot(*v as i64, *t);
                        }
                    }
                    return Err(release_all(built, &types));
                }
            },
        };
        built.push(slot);
    }
    Ok(built)
}

fn release_all(built: Vec<u64>, types: &[Type]) -> Wrong {
    for (slot, t) in built.into_iter().zip(types) {
        release_slot(slot as i64, *t);
    }
    Wrong
}
