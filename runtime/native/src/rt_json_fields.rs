// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The fields of a record or variant read from JSON (spec 2.12): each
//! from the text, or its default from the type's template, or `none`.
//! Split from `rt_json_decode.rs`, which reads every other kind of value.

use crate::heap::{obj, retain, slots};
use crate::rt_json_decode::{Failed, IGNORE, decode_at, drop_all, stop_at};
use crate::shapes::shape;
use kavach_kir::{FieldShape, Shape, Type};
use kavach_runtime_core::json::{Tree, expected, path_field};

/// The fields of a record (of type `record`, which may have a template)
/// or variant: each from `given`, or its default, or `none`.
pub(crate) fn decode_fields(
    given: &[(String, Tree)],
    fields: &[FieldShape],
    record: Type,
    path: &str,
    templates: i64,
    nodes: &[usize],
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
            Some((_, t)) => {
                let node = nodes.get(i).copied().unwrap_or(0);
                decode_at(t, f.ty, &path_field(path, &f.name), templates, node)
            }
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
