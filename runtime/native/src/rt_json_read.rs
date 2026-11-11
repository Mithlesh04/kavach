//! Reading `Json` values (spec 2.12) without making their lists and maps
//! (decision 0032): `j["key"]`, `j[i]` and the methods `list`, `object`,
//! `int`, `float`, `text`, `bool`, `is_null` and `len` in native code. On
//! a value from a parsed document they read its tape; on a value a
//! program made, they do what `stdlib/json/json.kav` does.

use crate::heap::{Data, alloc, data, obj, release, retain, slots, tag_of};
use crate::rt::type_of;
use crate::rt_json_tape::{ARRAY, Doc, NUMBER, OBJECT, kind, null, tags, value};
use crate::shapes::{len_of, with_str};
use kavach_kir::RtOp;
use std::cell::Cell;

/// How many recent keys remember where they were last found.
const PLACES: usize = 64;

/// The largest finite `Float`: `float()` gives `none` past it.
const LARGEST: f64 = f64::MAX;

thread_local! {
    /// Where each recent constant key (by its address) was last found in
    /// an object: objects in one array tend to list their fields in the
    /// same order, so the next one is looked at there first.
    static PLACE: [Cell<(usize, usize)>; PLACES] = const { [const { Cell::new((0, 0)) }; PLACES] };
}

/// Field `key` of object `w` of `doc`. `hint` is the key's address when
/// it is a constant, for remembering where it was found.
fn find(doc: &Doc, w: u64, key: &str, hint: usize) -> Option<u64> {
    if kind(w) != OBJECT {
        return None;
    }
    let fields = doc.run(w);
    let n = fields.len() / 2;
    let at = (hint >> 3) % PLACES;
    let start = PLACE.with(|p| match p[at].get() {
        (h, i) if h == hint && i < n => i,
        _ => 0,
    });
    let k = (start..n)
        .chain(0..start)
        .find(|k| doc.is(fields[2 * k], key.as_bytes()))?;
    if hint != 0 {
        PLACE.with(|p| p[at].set((hint, k)));
    }
    Some(fields[2 * k + 1])
}

/// `j[key]` (`j` borrowed): the field, or `Null`.
pub fn get(j: i64, key: &str, hint: usize) -> i64 {
    let json = obj(j).ty();
    if let Data::Json(doc, w, _) = &*data(j).borrow() {
        return find(doc, *w, key, hint).map_or_else(|| null(json), |v| value(doc, v, json));
    }
    if tag_of(j) == tags(json)[5] {
        let found = crate::rt_maps::get_text(slots(j)[0] as i64, key);
        if found != 0 {
            return found;
        }
    }
    null(json)
}

/// `j[i]` (`j` borrowed): the item, or `Null`.
pub fn at(j: i64, i: i64) -> i64 {
    let json = obj(j).ty();
    if let Data::Json(doc, w, _) = &*data(j).borrow() {
        let item = (kind(*w) == ARRAY)
            .then(|| doc.run(*w))
            .and_then(|items| usize::try_from(i).ok().and_then(|i| items.get(i)));
        return item.map_or_else(|| null(json), |v| value(doc, *v, json));
    }
    if tag_of(j) == tags(json)[4] {
        let items = crate::shapes::items_of(slots(j)[0] as i64);
        if let Some(v) = usize::try_from(i).ok().and_then(|i| items.get(i)) {
            retain(*v as i64);
            return *v as i64;
        }
    }
    null(json)
}

/// `j.list()` (`object` false) or `j.object()` (`j` borrowed): its list
/// or map (retained), or 0 for `none`.
pub fn items(j: i64, object: bool) -> i64 {
    let tag = tags(obj(j).ty())[if object { 5 } else { 4 }];
    if tag_of(j) != tag {
        return 0;
    }
    let made = match &*data(j).borrow() {
        Data::Json(..) => None,
        _ => Some(slots(j)[0] as i64),
    };
    let p = made.unwrap_or_else(|| crate::rt_json_tape::field(j));
    retain(p);
    p
}

/// The text of `Number` (variant 2) or `Text` (3) `j` (borrowed), given
/// to `f`. Values on a tape are arrays and objects, so these are made.
fn scalar<R>(j: i64, variant: usize, f: impl FnOnce(&str) -> R) -> Option<R> {
    (tag_of(j) == tags(obj(j).ty())[variant] && !crate::heap::has_data(j))
        .then(|| with_str(slots(j)[0] as i64, f))
}

/// `int()` (or `float()`, as bits, with `float`) of digits `d`.
fn number(d: &str, float: bool) -> Option<i64> {
    if float {
        crate::rt_numbers::float_of(d)
            .filter(|f| (-LARGEST..=LARGEST).contains(f))
            .map(|f| f.to_bits() as i64)
    } else {
        d.parse::<i64>().ok()
    }
}

/// `j.int() ?? fallback` (or `j.float()`), `j` borrowed.
fn number_or(j: i64, float: bool, fallback: i64) -> i64 {
    scalar(j, 2, |d| number(d, float))
        .flatten()
        .unwrap_or(fallback)
}

/// `j.len()` (`j` borrowed): its items or fields, otherwise 0.
fn len(j: i64) -> i64 {
    if let Data::Json(doc, w, _) = &*data(j).borrow() {
        let n = doc.run(*w).len();
        return (if kind(*w) == OBJECT { n / 2 } else { n }) as i64;
    }
    let t = tags(obj(j).ty());
    if tag_of(j) == t[4] || tag_of(j) == t[5] {
        let n = len_of(slots(j)[0] as i64);
        return (if tag_of(j) == t[5] { n / 2 } else { n }) as i64;
    }
    0
}

/// `j[key].int() ?? fallback` (`float` 0) or `.float()` (1) with a
/// constant key: what generated code calls, with `j` borrowed and the key
/// as UTF-8 bytes. Nothing is made for the field.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C-unwind" fn kv_json_get_number_or(
    j: i64,
    ptr: *const u8,
    len: usize,
    fallback: i64,
    float: i64,
) -> i64 {
    // SAFETY: generated code passes a constant text's bytes and length.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    // SAFETY: a constant text of the program is UTF-8.
    let key = unsafe { std::str::from_utf8_unchecked(bytes) };
    if let Data::Json(doc, w, _) = &*data(j).borrow() {
        return match find(doc, *w, key, ptr as usize) {
            Some(v) if kind(v) == NUMBER => number(doc.str(v), float != 0).unwrap_or(fallback),
            _ => fallback,
        };
    }
    let field = get(j, key, ptr as usize);
    let n = number_or(field, float != 0, fallback);
    release(field);
    n
}

/// `j[key]` with a constant key: what generated code calls, with `j`
/// borrowed and the key as UTF-8 bytes.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C-unwind" fn kv_json_get_text(j: i64, ptr: *const u8, len: usize) -> i64 {
    // SAFETY: generated code passes a constant text's bytes and length.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    // SAFETY: a constant text of the program is UTF-8.
    let key = unsafe { std::str::from_utf8_unchecked(bytes) };
    get(j, key, ptr as usize)
}

/// The JSON reading ops, with their operands owned; `code` is the
/// result's type.
pub fn read_op(op: RtOp, a: i64, b: i64, code: i64) -> i64 {
    let some = |slot: Option<i64>| slot.map_or(0, |s| alloc(type_of(code), Data::Boxed(s as u64)));
    let t = tags(obj(a).ty());
    let result = match op {
        RtOp::JsonGet => {
            let v = with_str(b, |k| get(a, k, 0));
            release(b);
            v
        }
        RtOp::JsonAt => at(a, b),
        RtOp::JsonList => items(a, false),
        RtOp::JsonObject => items(a, true),
        RtOp::JsonInt => some(scalar(a, 2, |d| number(d, false)).flatten()),
        RtOp::JsonFloat => some(scalar(a, 2, |d| number(d, true)).flatten()),
        RtOp::JsonIntOr => number_or(a, false, b),
        RtOp::JsonFloatOr => number_or(a, true, b),
        RtOp::JsonText => match tag_of(a) == t[3] {
            true => {
                retain(slots(a)[0] as i64);
                slots(a)[0] as i64
            }
            false => 0,
        },
        RtOp::JsonBool => some((tag_of(a) == t[1]).then(|| slots(a)[0] as i64)),
        RtOp::JsonIsNull => i64::from(tag_of(a) == t[0]),
        _ => len(a),
    };
    release(a);
    result
}
