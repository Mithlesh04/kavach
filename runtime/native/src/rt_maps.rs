//! Maps, `Shared` values and `list[i] = v` (spec 2.2, 6.2).
//!
//! A map keeps its entries in a hash table that remembers insertion order
//! (`table.rs`).

use crate::display::{equal, hash_slot};
use crate::heap::{
    Data, alloc, alloc_slots, items_of, len_of, obj, release, release_slot, retain, shape, slot_at,
    unique,
};
use crate::rt::{panic_with, type_of};
use crate::rt_data::optional;
use kavach_kir::{RtOp, Shape, Type};

/// `list[i] = v`: a copy of the list (unless unshared) with item `i` set.
pub fn list_set(list: i64, i: i64, value: i64, line: i32) -> i64 {
    let q = unique(list);
    let o = obj(q);
    let item = match shape(o.ty()) {
        Shape::List(t) => *t,
        _ => Type::I64,
    };
    let n = crate::heap::len_of(q) as i64;
    if i < 0 || i >= n {
        panic_with(
            &kavach_runtime_core::list_index_panic(i128::from(i), n as usize),
            line,
        )
    }
    if let w @ 1.. = crate::flat::width(o.ty()) {
        // A flat list: the item's fields are copied in.
        let at = i as usize * w;
        crate::flat::store(&mut crate::heap::slots_mut(q)[at..at + w], value);
        return q;
    }
    let old = crate::heap::with_slots(q, |s| std::mem::replace(&mut s[i as usize], value as u64));
    release_slot(old as i64, item);
    q
}

fn key_value_types(map: i64) -> (Type, Type) {
    match shape(obj(map).ty()) {
        Shape::Map(k, v) => (*k, *v),
        Shape::Set(t) => (*t, Type::Unit),
        _ => (Type::I64, Type::I64),
    }
}

/// Whether keys of type `kt` use the table's integer hashing: integers,
/// booleans and characters (equal exactly when their slots are).
fn int_key(kt: Type) -> bool {
    !kt.is_ref() && !kt.is_float()
}

/// The entry number of `key` in `map` (both borrowed). A lookup that saw a
/// long probe switches the map to the strong hash (`table.rs`).
pub(crate) fn find(map: i64, key: i64, kt: Type) -> Option<usize> {
    let cell = crate::heap::data(map);
    let (found, switch) = match &*cell.borrow() {
        // Floats are compared by value: 0.0 and -0.0 are the same key.
        Data::Map(t) if !int_key(kt) => {
            let hash = hash_slot(key, kt);
            (t.find(hash, |k| equal(k as i64, key, kt)), false)
        }
        Data::Map(t) => (t.find_int(key as u64), t.wants_strong()),
        _ => (None, false),
    };
    if switch && let Data::Map(t) = &mut *cell.borrow_mut() {
        t.make_strong();
    }
    found
}

/// A new, empty map of type `code`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_map_new(code: i64) -> i64 {
    alloc(type_of(code), Data::Map(Box::default()))
}

/// `map[key] ?? fallback` for an integer key and a scalar value, with
/// `map` borrowed: what generated code calls, without `kv_rt`'s dispatch
/// or reference counting.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_map_get_int(map: i64, key: i64, fallback: i64) -> i64 {
    let cell = crate::heap::data(map);
    let (value, switch) = match &*cell.borrow() {
        Data::Map(t) => (t.get_int(key as u64).map(|v| v as i64), t.wants_strong()),
        _ => (None, false),
    };
    if switch && let Data::Map(t) = &mut *cell.borrow_mut() {
        t.make_strong();
    }
    value.unwrap_or(fallback)
}

/// `map[key]` for a text key and a value held by reference, with `map`
/// borrowed and the key given as UTF-8 bytes: the value (retained), or 0
/// for `none`. What generated code calls for `j["name"]`: no text object
/// is made for the key, and nothing is dispatched on the map's type.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_map_get_text(map: i64, ptr: *const u8, len: usize) -> i64 {
    // The key is the program's constant: its address names its text for
    // the whole run, so its hash and where it was found last are kept by
    // address. Reading JSON objects of one shape, the key is there again.
    let at = (ptr as usize >> 3) % KEYS_SEEN;
    let (key, (hash, hint)) = SEEN.with(|seen| {
        let (was, known) = seen[at].get();
        let key = crate::rt::text(ptr, len);
        if was == (ptr as usize, len) {
            (key, known)
        } else {
            (key, (text_hash(key), usize::MAX))
        }
    });
    let same = |k: u64| crate::heap::with_str(k as i64, |s| s == key);
    let found = match &*crate::heap::data(map).borrow() {
        Data::Map(t) => t.find_from(hint, hash, same).map(|e| {
            let value = t.slots[e * 2 + 1] as i64;
            retain(value);
            (value, e)
        }),
        _ => None,
    };
    let Some((value, e)) = found else { return 0 };
    if e != hint {
        SEEN.with(|seen| seen[at].set(((ptr as usize, len), (hash, e))));
    }
    value
}

/// How many constant keys `kv_map_get_text` keeps: a slot each, by address.
const KEYS_SEEN: usize = 64;

thread_local! {
    /// Each constant key's address and length, its hash, and the entry it
    /// was found at.
    static SEEN: [Seen; KEYS_SEEN] = const { [const { std::cell::Cell::new(((0, 0), (0, usize::MAX))) }; KEYS_SEEN] };
}

type Seen = std::cell::Cell<((usize, usize), (u64, usize))>;

/// The hash `hash_slot` gives a text key.
fn text_hash(key: &str) -> u64 {
    let mut h = crate::hashing::FastHasher::new();
    std::hash::Hash::hash(key, &mut h);
    std::hash::Hasher::finish(&h)
}

/// `map[key]` like `kv_map_get_text`, with the key a borrowed text object.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_map_get_str(map: i64, key: i64) -> i64 {
    crate::heap::with_str(key, |k| get_text(map, k))
}

fn get_text(map: i64, key: &str) -> i64 {
    let hash = text_hash(key);
    let same = |k: u64| crate::heap::with_str(k as i64, |s| s == key);
    match &*crate::heap::data(map).borrow() {
        Data::Map(t) => t.find(hash, same).map_or(0, |e| {
            let value = t.slots[e * 2 + 1] as i64;
            retain(value);
            value
        }),
        _ => 0,
    }
}

/// `map[key] = value` (all three owned): what generated code calls.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_map_insert(map: i64, key: i64, value: i64) -> i64 {
    insert(map, key, value)
}

/// Sets `key` to `value` in `map` (all three owned); returns the map.
fn insert(map: i64, key: i64, value: i64) -> i64 {
    let q = unique(map);
    let (kt, vt) = key_value_types(q);
    if int_key(kt) {
        // One probe finds the key or the place for it.
        let old = match &mut *crate::heap::data(q).borrow_mut() {
            Data::Map(table) => table.insert_int(key as u64, value as u64),
            _ => None,
        };
        if let Some(old) = old {
            release_slot(old as i64, vt);
        }
        return q;
    }
    let found = find(q, key, kt);
    if let Data::Map(table) = &mut *crate::heap::data(q).borrow_mut() {
        match found {
            Some(e) => {
                let old = std::mem::replace(&mut table.slots[e * 2 + 1], value as u64);
                release_slot(old as i64, vt);
                release_slot(key, kt);
            }
            None => {
                table.push(hash_slot(key, kt), key as u64, value as u64);
            }
        }
    }
    q
}

/// Removes `key` from `map` (both owned); returns the map.
fn remove(map: i64, key: i64) -> i64 {
    let q = unique(map);
    let (kt, vt) = key_value_types(q);
    if let Some(e) = find(q, key, kt) {
        let removed = match &mut *crate::heap::data(q).borrow_mut() {
            Data::Map(table) => Some(table.remove(e)),
            _ => None,
        };
        if let Some((k, v)) = removed {
            release_slot(k as i64, kt);
            release_slot(v as i64, vt);
        }
    }
    release_slot(key, kt);
    q
}

/// A new list of type `code` holding `items` (already owned).
fn new_list(items: Vec<u64>, code: i64) -> i64 {
    alloc_slots(type_of(code), 0, items)
}

/// Map and `Shared` operations: `a` is the map or shared value.
pub fn map_or_shared(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    if op == RtOp::SharedNew {
        return crate::send::new_shared(a as u64, type_of(code), line);
    }
    if op == RtOp::MapInsert {
        return insert(a, b, c);
    }
    // Only a hint, for integer keys (`kir/src/prefetch.rs`).
    if op == RtOp::MapPrefetch {
        if let Data::Map(t) = &*crate::heap::data(a).borrow() {
            t.prefetch_int(b as u64);
        }
        release(a);
        return 0;
    }
    // A `Shared` value is seen by other tasks: its turn first.
    if matches!(op, RtOp::SharedGet | RtOp::SharedSet | RtOp::SharedLock) {
        crate::rt_tasks::turn();
    }
    if op == RtOp::MapRemove {
        return remove(a, b);
    }
    let (kt, vt) = key_value_types(a);
    let owned = |slot: u64, t: Type| {
        if t.is_ref() {
            retain(slot as i64);
        }
        slot
    };
    let result = match op {
        RtOp::MapGet | RtOp::MapContains => {
            let found = find(a, b, kt);
            release_slot(b, kt);
            if op == RtOp::MapContains {
                found.is_some() as i64
            } else {
                let value = found.map(|e| owned(slot_at(a, e * 2 + 1).0 as u64, vt) as i64);
                optional(value, code)
            }
        }
        RtOp::MapGetOr => {
            let found = find(a, b, kt);
            release_slot(b, kt);
            match found {
                Some(e) => {
                    release_slot(c, vt);
                    owned(slot_at(a, e * 2 + 1).0 as u64, vt) as i64
                }
                None => c,
            }
        }
        RtOp::MapLen => (len_of(a) / 2) as i64,
        RtOp::MapIsEmpty => (len_of(a) == 0) as i64,
        RtOp::MapKeys | RtOp::MapValues => {
            let slots = items_of(a);
            let (start, t) = if op == RtOp::MapKeys {
                (0, kt)
            } else {
                (1, vt)
            };
            let items = slots
                .iter()
                .skip(start)
                .step_by(2)
                .map(|s| owned(*s, t))
                .collect();
            new_list(items, code)
        }
        RtOp::MapEntries => {
            let pair = match shape(type_of(code)) {
                Shape::List(t) => *t,
                _ => Type::Unit,
            };
            let items = items_of(a)
                .chunks(2)
                .map(|e| alloc_slots(pair, 0, vec![owned(e[0], kt), owned(e[1], vt)]) as u64)
                .collect();
            new_list(items, code)
        }
        RtOp::SharedGet => {
            let inner = match shape(obj(a).ty()) {
                Shape::Shared(t) => *t,
                _ => Type::I64,
            };
            crate::send::shared_value(a, inner) as i64
        }
        RtOp::SharedLock | RtOp::SharedUnlock => {
            crate::send::shared_lock(a, op == RtOp::SharedLock);
            0
        }
        _ => {
            // SharedSet: every copy sees the change, so no copy-on-write.
            let inner = match shape(obj(a).ty()) {
                Shape::Shared(t) => *t,
                _ => Type::I64,
            };
            let old = crate::send::set_shared(a, b as u64, inner);
            release_slot(old as i64, inner);
            0
        }
    };
    release(a);
    result
}
