//! The program's heap types, and reading values out of objects: text,
//! lengths and slots. The objects themselves are in `heap.rs`.

use crate::heap::{Data, alloc, data, obj, release, retain, slot_area, slots};
use kavach_kir::{Shape, Type};
use std::sync::OnceLock;

static SHAPES: OnceLock<Vec<Shape>> = OnceLock::new();

/// For each heap type: whether its objects can hold references.
static HOLDS_REFS: OnceLock<Vec<bool>> = OnceLock::new();

pub fn set_shapes(shapes: &[Shape]) {
    let _ = SHAPES.set(shapes.to_vec());
    crate::flat::set_widths(shapes);
    let refs = shapes
        .iter()
        .enumerate()
        .map(|(id, s)| match s {
            // A flat list holds only numbers (`flat.rs`).
            Shape::List(_) if kavach_kir::flat_width(shapes, Type::Ref(id as u32)) > 0 => false,
            Shape::List(t) | Shape::Shared(t) | Shape::Set(t) | Shape::Optional(t) => t.is_ref(),
            Shape::Map(k, v) => k.is_ref() || v.is_ref(),
            Shape::Tuple(ts) | Shape::Closure(ts) => ts.iter().any(|t| t.is_ref()),
            Shape::Record { fields, .. } => fields.iter().any(|f| f.ty.is_ref()),
            Shape::Union { variants, .. } => variants
                .iter()
                .any(|v| v.fields.iter().any(|f| f.ty.is_ref())),
            Shape::Str | Shape::Bytes | Shape::Decimal | Shape::BigInt | Shape::Regex => false,
            _ => true,
        })
        .collect();
    let _ = HOLDS_REFS.set(refs);
}

/// The first heap type with this shape, such as the program's text type.
pub fn first_of(wanted: &Shape) -> Type {
    let shapes = SHAPES.get().expect("shapes are set before running");
    Type::Ref(shapes.iter().position(|s| s == wanted).unwrap_or(0) as u32)
}

/// The union named `name` whose first variant is `first`: a built-in
/// union such as `JsonError` (spec 2.12), when the program uses it.
pub fn find_union(name: &str, first: &str) -> Option<Type> {
    let shapes = SHAPES.get()?;
    let id = shapes.iter().position(|s| {
        matches!(s, Shape::Union { name: n, variants }
            if n == name && variants.first().is_some_and(|v| v.name == first))
    })?;
    Some(Type::Ref(id as u32))
}

pub fn shape(ty: Type) -> &'static Shape {
    let Type::Ref(id) = ty else {
        return &Shape::Str;
    };
    &SHAPES.get().expect("shapes are set before running")[id as usize]
}

pub(crate) fn holds_refs(ty: Type) -> bool {
    match ty {
        Type::Ref(id) => HOLDS_REFS.get().is_none_or(|r| r[id as usize]),
        _ => false,
    }
}

pub fn new_str(text: String, ty: Type) -> i64 {
    alloc(ty, Data::Str(text.into()))
}

/// New text from a borrowed `&str`: short text needs no heap buffer.
pub fn new_text(text: &str, ty: Type) -> i64 {
    alloc(ty, Data::Str(crate::text::Text::of(text)))
}

pub fn str_of(p: i64) -> String {
    with_str(p, str::to_string)
}

/// Calls `f` with the text in `p` (borrowed, not copied).
pub fn with_str<R>(p: i64, f: impl FnOnce(&str) -> R) -> R {
    match &*data(p).borrow() {
        Data::Str(s) => f(s.as_str()),
        _ => f(""),
    }
}

/// The error inside any `context` wrappers (borrowed, like `p`).
pub fn innermost(p: i64) -> i64 {
    match &*data(p).borrow() {
        Data::Context(c) => innermost(c.1 as i64),
        _ => p,
    }
}

/// The number of slots of a tuple, record, map or set, or of items of a
/// list.
pub fn len_of(p: i64) -> usize {
    if crate::flat::packed(obj(p).ty()) {
        return crate::heap::list_len_items(p);
    }
    match &*data(p).borrow() {
        Data::Map(t) => t.slots.len(),
        _ => slot_area(p).1 / crate::flat::width(obj(p).ty()).max(1),
    }
}

/// Slot `i` of a list, tuple, record, map or set (borrowed), and its type.
pub fn slot_at(p: i64, i: usize) -> (i64, Type) {
    let ty = slot_type(p, i);
    if crate::flat::packed(obj(p).ty()) {
        return (crate::flat::slot_of(slots(p), i, ty) as i64, ty);
    }
    if let Data::Map(t) = &*data(p).borrow() {
        return (t.slots[i] as i64, ty);
    }
    (slots(p)[i] as i64, ty)
}

/// Item `i` of a list, or slot `i` of a tuple, record, map or set, as
/// an owned value: retained, or boxed from a flat list.
pub fn item_owned(p: i64, i: usize) -> i64 {
    let ty = obj(p).ty();
    match crate::flat::width(ty) {
        0 => {
            let (slot, t) = slot_at(p, i);
            if t.is_ref() {
                retain(slot);
            }
            slot
        }
        w => crate::flat::box_item(crate::flat::item_type(ty), &slots(p)[i * w..(i + 1) * w]),
    }
}

/// The items of a list, or slots of a tuple, record, map or set: a copy,
/// borrowed from `p`. A flat list's items are boxed into objects, which
/// are released when this is dropped.
pub struct Items {
    slots: Vec<u64>,
    boxed: bool,
}

impl std::ops::Deref for Items {
    type Target = [u64];
    fn deref(&self) -> &[u64] {
        &self.slots
    }
}

impl<'a> IntoIterator for &'a Items {
    type Item = &'a u64;
    type IntoIter = std::slice::Iter<'a, u64>;
    fn into_iter(self) -> Self::IntoIter {
        self.slots.iter()
    }
}

impl Drop for Items {
    fn drop(&mut self) {
        if self.boxed {
            self.slots.iter().for_each(|s| release(*s as i64));
        }
    }
}

pub fn items_of(p: i64) -> Items {
    if let Data::Map(t) = &*data(p).borrow() {
        let slots = t.slots.clone();
        return Items {
            slots,
            boxed: false,
        };
    }
    let ty = obj(p).ty();
    if crate::flat::packed(ty) {
        return Items {
            slots: crate::flat::unpack(
                slots(p),
                crate::heap::list_len_items(p),
                crate::flat::item_of(ty),
            ),
            boxed: false,
        };
    }
    match crate::flat::width(ty) {
        0 => Items {
            slots: slots(p).to_vec(),
            boxed: false,
        },
        w => Items {
            slots: crate::flat::box_all(crate::flat::item_type(ty), slots(p), w),
            boxed: true,
        },
    }
}

/// The type of slot `i` of object `p`.
pub(crate) fn slot_type(p: i64, i: usize) -> Type {
    let o = obj(p);
    match shape(o.ty()) {
        // A list of text: its words are not slots (see `text_items.rs`).
        Shape::List(_) if crate::flat::is_text(o.ty()) => Type::I64,
        // A flat list's slots are the fields of its items.
        Shape::List(t) if crate::flat::width(o.ty()) > 0 => {
            crate::flat::field_type(*t, i % crate::flat::width(o.ty()))
        }
        Shape::List(t) => *t,
        Shape::Map(k, v) => [*k, *v][i % 2],
        Shape::Set(t) => [*t, Type::Unit][i % 2],
        // A closure: the function's address, then its captured values.
        Shape::Closure(ts) if i > 0 => ts[i - 1],
        Shape::Tuple(ts) => ts[i],
        // An interface value: the concrete value (its type in slot 1),
        // the type slot itself, then the method closures (spec 2.15).
        Shape::Iface { methods, .. } => match i {
            0 => crate::rt::type_of(crate::heap::slots(p)[1] as i64),
            1 => Type::I64,
            _ => methods[i - 2].1,
        },
        Shape::Record { fields, .. } => fields[i].ty,
        Shape::Union { variants, .. } => variants[o.tag as usize].fields[i].ty,
        _ => Type::I64,
    }
}

/// The type of each slot inside object `p`, for releasing and printing.
pub fn slot_types(p: i64) -> Vec<Type> {
    (0..len_of(p)).map(|i| slot_type(p, i)).collect()
}
