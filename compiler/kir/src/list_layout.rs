// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! How a list lays out its items in machine code: flat records and
//! tuples, packed small numbers, and text kept inside its two words
//! (`docs/architecture/flat-lists.md`). Split from `ir.rs`.

use crate::ir::{Shape, Type};

/// The most fields an item of a flat list may have.
pub const FLAT_MAX: usize = 8;

/// How many words each item of list type `list` takes when the list is
/// flat, or 0. A list is flat when its items are records or tuples whose
/// fields are all scalars (numbers, booleans, characters): their fields
/// are stored in the list's buffer, not in an object per item
/// (`docs/architecture/flat-lists.md`).
pub fn flat_width(shapes: &[Shape], list: Type) -> usize {
    let Type::Ref(id) = list else { return 0 };
    let Some(Shape::List(Type::Ref(item))) = shapes.get(id as usize) else {
        return 0;
    };
    let fields: Vec<Type> = match &shapes[*item as usize] {
        Shape::Record { drop: true, .. } => return 0,
        Shape::Record { fields, .. } => fields.iter().map(|f| f.ty).collect(),
        Shape::Tuple(ts) => ts.clone(),
        _ => return 0,
    };
    let plain = !fields.is_empty() && fields.len() <= FLAT_MAX;
    if plain && fields.iter().all(|t| !t.is_ref()) {
        fields.len()
    } else {
        0
    }
}

/// The bytes a list packs each item of type `item` into (decisions 0030
/// and 0045), or 0 for one 64-bit slot. `Char` stays a slot: it is a
/// code point, not a number.
pub fn packed_size(item: Type) -> usize {
    match item {
        Type::U8 | Type::I8 | Type::Bool => 1,
        Type::U16 | Type::I16 => 2,
        Type::U32 | Type::I32 | Type::F32 => 4,
        _ => 0,
    }
}

/// The bytes per item of list type `list` when it packs them, or 0.
pub fn item_bytes(shapes: &[Shape], list: Type) -> usize {
    let Type::Ref(id) = list else { return 0 };
    match shapes.get(id as usize) {
        Some(Shape::List(item)) => packed_size(*item),
        _ => 0,
    }
}

/// Whether list type `list` holds text. Such a list keeps each item in
/// two words: text up to 15 bytes inside them, longer text as a pointer
/// to a text object (`docs/architecture/flat-lists.md`).
pub fn text_list(shapes: &[Shape], list: Type) -> bool {
    let Type::Ref(id) = list else { return false };
    matches!(shapes.get(id as usize), Some(Shape::List(Type::Ref(item)))
        if shapes.get(*item as usize) == Some(&Shape::Str))
}
