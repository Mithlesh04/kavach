//! More methods of text and lists (spec 14.2). Results must match the
//! interpreter (`compiler/interp/src/std_lib.rs`).

use crate::display::compare;
use crate::heap::{alloc_slots, items_of, new_str, obj, release, retain, shape, str_of, with_str};
use crate::rt::type_of;
use crate::rt_data::optional;
use kavach_kir::{RtOp, Shape, Type};
use std::cmp::Ordering;

/// The order of two slots of type `ty`: numbers, text, characters.
pub fn order(a: i64, b: i64, ty: Type) -> Ordering {
    match ty {
        // `NaN` sorts after every number, so a list holding one still sorts
        // (spec 14.2).
        Type::F64 | Type::F32 => {
            let wide = |s: i64| match ty {
                Type::F32 => f64::from(crate::display::f32_of(s)),
                _ => f64::from_bits(s as u64),
            };
            let (x, y) = (wide(a), wide(b));
            match (x.is_nan(), y.is_nan()) {
                (false, false) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                (nan_x, nan_y) => nan_x.cmp(&nan_y),
            }
        }
        Type::Ref(_) => compare(a, b),
        t if t.is_signed() => a.cmp(&b),
        _ => (a as u64).cmp(&(b as u64)),
    }
}

fn item_type(list: i64) -> Type {
    match shape(obj(list).ty()) {
        Shape::List(t) => *t,
        _ => Type::I64,
    }
}

/// `list.sort()` (`list` owned): sorted in place when nothing else holds
/// the list, otherwise in a copy. Equal integers can't be told apart, so
/// integer lists use a radix sort (`radix.rs`); other items keep their
/// order when equal (`0.0` and `-0.0`, text that compares equal).
fn sort(list: i64, item: Type) -> i64 {
    let q = crate::heap::unique(list);
    crate::heap::with_slots(q, |v| match item {
        t if t.is_signed() => crate::radix::sort(v, true),
        t if t.is_int() || t == Type::Bool || t == Type::Char => crate::radix::sort(v, false),
        _ => v.sort_by(|x, y| order(*x as i64, *y as i64, item)),
    });
    q
}

/// A new list of type `code` holding these slots, each retained.
fn list_of(slots: &[u64], item: Type, code: i64) -> i64 {
    if item.is_ref() {
        slots.iter().for_each(|s| retain(*s as i64));
    }
    alloc_slots(type_of(code), 0, slots.to_vec())
}

/// The text type that a list of text (`code`) holds.
fn text_item(code: i64) -> Type {
    match shape(type_of(code)) {
        Shape::List(t) | Shape::Optional(t) => *t,
        _ => type_of(code),
    }
}

/// `text.split(sep)` (both owned): the text is read in place, and a
/// one-byte separator is found with a byte search. Short parts go
/// straight into the list's items (`text_items.rs`), with no objects.
fn split(a: i64, b: i64, code: i64) -> i64 {
    let item = text_item(code);
    let sep = str_of(b);
    release(b);
    let words: Vec<u64> = with_str(a, |s| match sep.as_bytes() {
        // An ASCII byte never appears inside another character, so it
        // is counted first and the list is made the right size at once.
        &[byte] if byte.is_ascii() => {
            let n = s.bytes().filter(|b| *b == byte).count() + 1;
            crate::text_items::split_at_byte(s, byte, n, item)
        }
        _ => text_words(s.split(sep.as_str()), 0, item),
    });
    release(a);
    crate::heap::alloc_words(type_of(code), words)
}

/// The words of a list of text holding `parts` (about `n` of them).
fn text_words<'s>(parts: impl Iterator<Item = &'s str>, n: usize, item: Type) -> Vec<u64> {
    let mut words = Vec::with_capacity(n * crate::text_items::WIDTH);
    for part in parts {
        words.extend_from_slice(&crate::text_items::encode(part, item));
    }
    words
}

/// Text methods. `a` is the text (owned), `b` and `c` the arguments.
pub fn text_op(op: RtOp, a: i64, b: i64, c: i64, code: i64) -> i64 {
    if op == RtOp::StrSplit {
        return split(a, b, code);
    }
    let s = str_of(a);
    release(a);
    let take = |p: i64| {
        let t = str_of(p);
        release(p);
        t
    };
    match op {
        RtOp::StrReplace => {
            let (old, new) = (take(b), take(c));
            new_str(s.replace(&old, &new), type_of(code))
        }
        RtOp::StrLines => {
            let words = text_words(s.lines(), 0, text_item(code));
            crate::heap::alloc_words(type_of(code), words)
        }
        RtOp::StrSplit | RtOp::StrChars => {
            let slots: Vec<u64> = s.chars().map(|ch| ch as u64).collect();
            alloc_slots(type_of(code), 0, slots)
        }
        RtOp::StrIndexOf => {
            let part = take(b);
            let at = s.find(&part).map(|byte| s[..byte].chars().count() as i64);
            optional(at, code)
        }
        _ => {
            // StrSlice: characters `b..c`, clamped.
            let (start, end) = (b.max(0) as usize, c.max(0) as usize);
            let part: String = s
                .chars()
                .skip(start)
                .take(end.saturating_sub(start))
                .collect();
            new_str(part, type_of(code))
        }
    }
}

/// List methods. `a` is the list (owned), `b` an argument.
pub fn list_op(op: RtOp, a: i64, b: i64, code: i64) -> i64 {
    let item = item_type(a);
    if op == RtOp::ListSort {
        return sort(a, item);
    }
    if op == RtOp::ListJoin && crate::flat::is_text(obj(a).ty()) {
        // Read in place, without making an object per item.
        let sep = str_of(b);
        release(b);
        let joined = crate::text_items::join(crate::heap::slots(a), &sep);
        release(a);
        return crate::heap::alloc(type_of(code), crate::heap::Data::Str(joined));
    }
    let slots = items_of(a);
    let result = match op {
        RtOp::ListJoin => {
            let sep = str_of(b);
            release(b);
            // Each item is written straight into the result.
            let mut out = crate::text::Text::with_capacity(slots.len() * (sep.len() + 8));
            for (i, s) in slots.iter().enumerate() {
                if i > 0 {
                    out.push_str(&sep);
                }
                crate::display::append_display(&mut out, *s as i64, item);
            }
            crate::heap::alloc(type_of(code), crate::heap::Data::Str(out))
        }
        RtOp::ListSortBy => {
            // `b` holds the keys, in the same order as the items.
            let keys = items_of(b);
            let key_ty = item_type(b);
            let mut index: Vec<usize> = (0..slots.len()).collect();
            index.sort_by(|x, y| order(keys[*x] as i64, keys[*y] as i64, key_ty));
            release(b);
            let sorted: Vec<u64> = index.iter().map(|i| slots[*i]).collect();
            list_of(&sorted, item, code)
        }
        RtOp::ListReverse => {
            let reversed: Vec<u64> = slots.iter().rev().copied().collect();
            list_of(&reversed, item, code)
        }
        RtOp::ListTake | RtOp::ListSkip => {
            let n = (b.max(0) as usize).min(slots.len());
            let part = if op == RtOp::ListTake {
                &slots[..n]
            } else {
                &slots[n..]
            };
            list_of(part, item, code)
        }
        RtOp::ListIndexOf => {
            let at = slots
                .iter()
                .position(|s| crate::display::equal(*s as i64, b, item));
            crate::heap::release_slot(b, item);
            optional(at.map(|i| i as i64), code)
        }
        _ => {
            // ListMin and ListMax: the first of the smallest or largest.
            let pick = slots.iter().copied().reduce(|best, s| {
                let o = order(s as i64, best as i64, item);
                let better = if op == RtOp::ListMin {
                    o.is_lt()
                } else {
                    o.is_gt()
                };
                if better { s } else { best }
            });
            let pick = pick.map(|s| {
                if item.is_ref() {
                    retain(s as i64);
                }
                s as i64
            });
            optional(pick, code)
        }
    };
    release(a);
    result
}
