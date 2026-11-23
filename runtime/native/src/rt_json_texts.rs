// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.decode_texts<T>(names, texts)` and `json.encode_texts(value)` (spec
//! 2.12) in native code, as the interpreter does them
//! (`compiler/interp/src/json_texts.rs`): each text read by its field's
//! type (`kavach_runtime_core::json::text_tree`), then decoded as
//! `json.decode` decodes; and a record's fields as `print` writes them.

use crate::display::display;
use crate::heap::{obj, release, slots};
use crate::rt::{panic_with, type_of};
use crate::rt_errors::set_error;
use crate::rt_json_decode::{IGNORE, decode_every};
use crate::shapes::{items_of, new_str, shape, with_str};
use kavach_kir::{Shape, Type};
use kavach_runtime_core::json::{TextKind, Tree, path_field, text_tree};

/// How a field of type `ty` reads its text.
fn kind_of(ty: Type) -> TextKind {
    match ty {
        Type::F64 => TextKind::Float,
        Type::F32 => TextKind::Float32,
        Type::Bool => TextKind::Bool,
        t if t.is_int() => TextKind::Whole,
        Type::Ref(_) => match shape(ty) {
            Shape::Optional(inner) => kind_of(*inner),
            Shape::Decimal => TextKind::Decimal,
            Shape::BigInt => TextKind::Whole,
            _ => TextKind::Text,
        },
        _ => TextKind::Text,
    }
}

/// `json.decode_texts`: names `a`, texts `b` and templates `c`, all
/// owned; a value of type `code`, or 0 with the error set.
pub fn decode_texts(a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    let ty = type_of(code);
    let (names, texts) = (items_of(a), items_of(b));
    if names.len() != texts.len() {
        let why = format!(
            "`json.decode_texts` was given {} names and {} texts",
            names.len(),
            texts.len()
        );
        panic_with(&why, line);
    }
    let fields = match shape(ty) {
        Shape::Record { fields, .. } => fields.clone(),
        _ => Vec::new(),
    };
    let mut object = Vec::with_capacity(names.len());
    let mut problems = Vec::new();
    for (name, text) in names.iter().zip(texts.iter()) {
        let name = with_str(*name as i64, str::to_string);
        let kind =
            (fields.iter().find(|f| f.name == name)).map_or(TextKind::Text, |f| kind_of(f.ty));
        let value = match *text {
            0 => Tree::Null,
            t => with_str(t as i64, |s| text_tree(kind, s)).unwrap_or_else(|want| {
                problems.push((path_field("", &name), want.to_string()));
                Tree::Null
            }),
        };
        object.push((name, value));
    }
    let result = if problems.is_empty() {
        IGNORE.with(|i| i.set(false));
        let decoded = decode_every(&Tree::Object(object), ty, c);
        IGNORE.with(|i| i.set(true));
        decoded
    } else {
        Err(problems)
    };
    release(a);
    release(b);
    release(c);
    match result {
        Ok(slot) => slot as i64,
        Err(problems) => {
            set_error(crate::rt_json::invalid(&problems));
            0
        }
    }
}

/// Whether `t` is an optional type.
fn optional(t: Type) -> bool {
    matches!(t, Type::Ref(_)) && matches!(shape(t), Shape::Optional(_))
}

/// `json.encode_texts`: the value in tuple `a` (owned); a list of type `code`.
pub fn texts(a: i64, code: i64) -> i64 {
    let list = type_of(code);
    let text_ty = match shape(list) {
        Shape::List(item) => match shape(*item) {
            Shape::Optional(inner) => *inner,
            _ => *item,
        },
        _ => Type::Unit,
    };
    let item = match shape(obj(a).ty()) {
        Shape::Tuple(ts) => ts[0],
        _ => Type::Unit,
    };
    let value = slots(a)[0] as i64;
    let shown = |slot: u64, t: Type| new_str(display(slot as i64, t, true), text_ty) as u64;
    let out: Vec<u64> = match (item, shape(item)) {
        (Type::Ref(_), Shape::Record { fields, .. }) => (fields.iter().zip(slots(value)))
            .map(|(f, &slot)| match f.secret {
                true => new_str("<secret>".to_string(), text_ty) as u64,
                false if slot == 0 && optional(f.ty) => 0,
                false => shown(slot, f.ty),
            })
            .collect(),
        _ => vec![shown(value as u64, item)],
    };
    release(a);
    crate::heap::alloc_slots(list, 0, out)
}
