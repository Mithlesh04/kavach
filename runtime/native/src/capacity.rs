//! Lists, maps and sets made by `capacity(n)`: at most `n` entries, with
//! room for all of them made at once (spec 2.2). The limit is kept in
//! the object's header (`heap::Obj::limit`), and copies keep it
//! (`rc::unique`).

use crate::heap::{Data, alloc, len_of, limit_of, obj, release, set_limit, shape};
use crate::rt::{panic_with, type_of};
use kavach_kir::{RtOp, Shape};
use kavach_runtime_core::{MAX_CAPACITY, capacity_panic, full_panic};

/// `List.capacity(n, first)`, `Map.capacity(…)`, `Set.capacity(…)`, with
/// `first` 0 when not given; `List.is_full()` and `Map.is_full()` (sets
/// too); and `ListNoLimit`, `xs` for `xs + ys`, whose result has no limit.
pub fn capacity_op(op: RtOp, a: i64, b: i64, code: i64, line: i32) -> i64 {
    match op {
        RtOp::ListIsFull | RtOp::MapIsFull => {
            let full = limit_of(a).is_some_and(|n| entries(a) >= n);
            release(a);
            full as i64
        }
        RtOp::ListNoLimit => no_limit(a),
        RtOp::RingCapacity => ring(a, b, code, line),
        _ => {
            if a > MAX_CAPACITY {
                panic_with(&capacity_panic(kind(op), a, 0), line)
            }
            let n = a.max(0) as usize;
            let made = match (op, b) {
                (RtOp::ListCapacity, 0) => crate::heap::alloc_slots(type_of(code), 0, Vec::new()),
                (RtOp::ListCapacity, _) => crate::rc::unique(b),
                (RtOp::MapCapacity, 0) | (RtOp::SetCapacity, 0) => empty_map(code, n),
                (RtOp::MapCapacity, _) => crate::rc::unique(b),
                _ => set_of(b, code, n),
            };
            let first = entries(made);
            if first > n {
                panic_with(&capacity_panic(kind(op), a, first), line)
            }
            if op == RtOp::ListCapacity {
                crate::lists::room_for_limit(made, n);
            } else {
                set_limit(made, n);
            }
            made
        }
    }
}

/// `List`, `Map` or `Set`, for messages.
fn kind(op: RtOp) -> &'static str {
    match op {
        RtOp::ListCapacity => "List",
        RtOp::MapCapacity => "Map",
        _ => "Set",
    }
}

/// How many entries list, map or set `p` holds.
fn entries(p: i64) -> usize {
    match shape(obj(p).ty()) {
        Shape::Map(..) | Shape::Set(_) => len_of(p) / 2,
        _ => len_of(p),
    }
}

/// An empty map or set with room for `n` entries.
fn empty_map(code: i64, n: usize) -> i64 {
    let table = crate::table::Table::with_room(n);
    alloc(type_of(code), Data::Map(Box::new(table)))
}

/// A set of the items of list `list` (owned).
fn set_of(list: i64, code: i64, n: usize) -> i64 {
    let mut set = empty_map(code, n);
    for i in 0..len_of(list) {
        let item = crate::shapes::item_owned(list, i);
        set = crate::rt_maps::kv_map_insert(set, item, 0, 0);
    }
    release(list);
    set
}

/// `list` (owned) without its limit: itself when it has none, otherwise
/// a copy, which has none.
fn no_limit(list: i64) -> i64 {
    if limit_of(list).is_none() {
        return list;
    }
    crate::rc::retain(list);
    let copy = crate::rc::unique(list);
    crate::heap::obj_mut(copy).limit = 0;
    crate::heap::set_ring(copy, false);
    release(list);
    copy
}

/// `Ring.capacity(n, first)`: the last `n` items of `first` (owned; 0
/// for none), in a list that drops its oldest item when full.
fn ring(n: i64, first: i64, code: i64, line: i32) -> i64 {
    // Past the limit is a panic, as for a list (spec 2.2).
    if n > MAX_CAPACITY {
        panic_with(&capacity_panic("Ring", n, 0), line)
    }
    let n = n.max(0) as usize;
    let mut made = crate::heap::alloc_slots(type_of(code), 0, Vec::new());
    crate::lists::room_for_limit(made, n);
    crate::heap::set_ring(made, true);
    if first != 0 {
        for i in 0..len_of(first) {
            made = push(made, crate::shapes::item_owned(first, i), 0);
        }
        release(first);
    }
    made
}

/// `list.push(item)` (both owned) on its slow path: a full list with a
/// limit panics, a full ring drops its oldest item (spec 2.2).
pub fn push(list: i64, item: i64, line: i32) -> i64 {
    let full = limit_of(list).filter(|n| len_of(list) >= *n);
    if let Some(n) = full {
        if !crate::heap::is_ring(list) {
            panic_with(&full_panic("list", n), line)
        }
        let q = crate::rc::unique(list);
        if n == 0 {
            if let Shape::List(t) = shape(obj(q).ty()) {
                crate::rc::release_slot(item, *t);
            }
            return q;
        }
        crate::lists::drop_first(q);
        crate::heap::push_item(q, item as u64);
        return q;
    }
    let q = crate::rc::unique(list);
    crate::heap::push_item(q, item as u64);
    q
}

/// Panics when map or set `p` is full and `found` says `key` is new:
/// changing a key it holds is not adding one (spec 2.2).
pub fn check_insert(p: i64, found: impl FnOnce() -> bool, line: i32) {
    if let Some(n) = limit_of(p).filter(|n| len_of(p) / 2 >= *n)
        && !found()
    {
        let what = match shape(obj(p).ty()) {
            Shape::Set(_) => "set",
            _ => "map",
        };
        panic_with(&full_panic(what, n), line)
    }
}
