//! Flat lists: a list of plain records or tuples keeps their fields in
//! its own buffer, `width` words per item, instead of a pointer to an
//! object per item (`docs/architecture/flat-lists.md`).
//!
//! Generated code reads and writes the fields in place. The runtime's
//! generic code (printing, comparing, sorting, …) works on objects, so
//! here items are boxed into objects when it needs them and unboxed when
//! they go back into a list. Records and text have no identity, so a
//! boxed copy is the same value.
//!
//! A list of text is flat too, with items of its own kind
//! (`text_items.rs`).

use crate::heap::{alloc_fixed, obj, release, slots, slots_mut};
use crate::shapes::shape;
use crate::text_items;
use kavach_kir::{Shape, Type};
use std::sync::OnceLock;

/// For each heap type: the width of its items when it is a flat list
/// (of records or of text), otherwise 0; whether it is a list of text;
/// and the bytes per item when its items are packed (decisions 0030 and
/// 0045), otherwise 0.
static WIDTHS: OnceLock<Vec<(usize, bool, usize)>> = OnceLock::new();

pub fn set_widths(shapes: &[Shape]) {
    let widths = (0..shapes.len())
        .map(|id| {
            let ty = Type::Ref(id as u32);
            let packed = kavach_kir::item_bytes(shapes, ty);
            match kavach_kir::text_list(shapes, ty) {
                true => (text_items::WIDTH, true, 0),
                false => (kavach_kir::flat_width(shapes, ty), false, packed),
            }
        })
        .collect();
    let _ = WIDTHS.set(widths);
}

/// The bytes per item of `ty` when it is a list whose items are packed
/// (1, 2 or 4), otherwise 0.
pub fn item_size(ty: Type) -> usize {
    match (ty, WIDTHS.get()) {
        (Type::Ref(id), Some(w)) => w[id as usize].2,
        _ => 0,
    }
}

/// Whether `ty` is a list whose items are packed.
pub fn packed(ty: Type) -> bool {
    item_size(ty) > 0
}

/// How many 64-bit words hold `n` packed items of `size` bytes.
pub fn words_for(n: usize, size: usize) -> usize {
    (n * size).div_ceil(8)
}

/// Item `i`, of `size` bytes, of packed words, zero-extended.
pub fn get_item(words: &[u64], i: usize, size: usize) -> u64 {
    let (per, bits) = (8 / size, 8 * size);
    (words[i / per] >> (bits * (i % per))) & (u64::MAX >> (64 - bits))
}

/// Sets item `i`, of `size` bytes, of packed words to the low bytes of
/// `v`.
pub fn set_item(words: &mut [u64], i: usize, size: usize, v: u64) {
    let (per, bits) = (8 / size, 8 * size);
    let shift = bits * (i % per);
    let mask = (u64::MAX >> (64 - bits)) << shift;
    let w = &mut words[i / per];
    *w = (*w & !mask) | ((v << shift) & mask);
}

/// Packs one item per word into `size` bytes per item.
pub fn pack(items: &[u64], size: usize) -> Vec<u64> {
    let mut out = vec![0u64; words_for(items.len(), size)];
    for (i, item) in items.iter().enumerate() {
        set_item(&mut out, i, size, *item);
    }
    out
}

/// Item `i` of packed `words`, of type `item`, as a slot: a signed item
/// keeps its sign, as every slot of a signed type does, and a `Float32`
/// is its 32 bits.
pub fn slot_of(words: &[u64], i: usize, item: Type) -> u64 {
    let raw = get_item(words, i, kavach_kir::packed_size(item));
    match item {
        Type::I8 => raw as u8 as i8 as i64 as u64,
        Type::I16 => raw as u16 as i16 as i64 as u64,
        Type::I32 => raw as u32 as i32 as i64 as u64,
        _ => raw,
    }
}

/// Unpacks `n` packed items of type `item` into one item per word.
pub fn unpack(words: &[u64], n: usize, item: Type) -> Vec<u64> {
    (0..n).map(|i| slot_of(words, i, item)).collect()
}

/// The item type of list type `ty`.
pub fn item_of(ty: Type) -> Type {
    match shape(ty) {
        Shape::List(t) => *t,
        _ => Type::U8,
    }
}

/// The words per item of list type `ty`, or 0 when it is not flat.
pub fn width(ty: Type) -> usize {
    match (ty, WIDTHS.get()) {
        (Type::Ref(id), Some(w)) => w[id as usize].0,
        _ => 0,
    }
}

/// Whether `ty` is a list of text.
pub fn is_text(ty: Type) -> bool {
    match (ty, WIDTHS.get()) {
        (Type::Ref(id), Some(w)) => w[id as usize].1,
        _ => false,
    }
}

/// The item type of list type `ty`.
pub fn item_type(ty: Type) -> Type {
    match shape(ty) {
        Shape::List(t) => *t,
        _ => Type::I64,
    }
}

/// The type of field `i` of a flat list's item type.
pub fn field_type(item: Type, i: usize) -> Type {
    match shape(item) {
        Shape::Record { fields, .. } => fields[i].ty,
        Shape::Tuple(ts) => ts[i],
        _ => Type::I64,
    }
}

/// A new object of type `item` holding these fields (or text item).
pub fn box_item(item: Type, fields: &[u64]) -> i64 {
    if *shape(item) == Shape::Str {
        return text_items::boxed(fields, item);
    }
    let p = alloc_fixed(item, 0, fields.len());
    slots_mut(p).copy_from_slice(fields);
    p
}

/// Each item of flat list `words` (of `width`) as a new object.
pub fn box_all(item: Type, words: &[u64], width: usize) -> Vec<u64> {
    words
        .chunks(width)
        .map(|f| box_item(item, f) as u64)
        .collect()
}

/// Appends the fields of object `p` to `words`, consuming the reference.
pub fn unbox_into(p: i64, words: &mut Vec<u64>) {
    if is_text_object(p) {
        words.extend_from_slice(&text_items::encode_object(p));
    } else {
        words.extend_from_slice(slots(p));
        release(p);
    }
}

/// Stores object `p` (consumed) as the item at `dst`, after releasing
/// what the item held.
pub fn store(dst: &mut [u64], p: i64) {
    if is_text_object(p) {
        text_items::release_all(dst);
        dst.copy_from_slice(&text_items::encode_object(p));
    } else {
        dst.copy_from_slice(slots(p));
        release(p);
    }
}

fn is_text_object(p: i64) -> bool {
    *shape(obj(p).ty()) == Shape::Str
}

/// The words of a flat list holding these objects (whose references
/// are consumed), with a capacity that is a whole number of items.
pub fn unbox_all(items: Vec<u64>, width: usize) -> Vec<u64> {
    let mut words = Vec::with_capacity(items.len() * width);
    for p in items {
        unbox_into(p as i64, &mut words);
    }
    words
}

/// `words` with a capacity that is a whole number of items: a list
/// keeps its length and capacity in items.
pub fn fit(words: Vec<u64>, width: usize) -> Vec<u64> {
    if words.capacity().is_multiple_of(width) {
        return words;
    }
    // `with_capacity` gives exactly the capacity asked for.
    let mut fitted = Vec::with_capacity(words.capacity().div_ceil(width) * width);
    fitted.extend_from_slice(&words);
    fitted
}
