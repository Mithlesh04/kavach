//! Object lifetimes: reference counts, freeing, and copy-on-write
//! (`unique`). Split from `heap.rs`, which allocates and reads objects.

use crate::heap::{
    DATA, DATA_CLASS, Data, FIXED, LIST, ListObj, WithData, data, fixed_layout, list_mut,
    new_list_raw, new_obj, obj, obj_mut, slot_type, slots,
};
use crate::live::count_live;
use crate::shapes::{holds_refs, shape};
use kavach_kir::{Shape, Type};

pub(crate) fn free(p: i64) {
    count_live(-1);
    let o = obj(p);
    // SAFETY: called once, when the last reference is released; each kind
    // is taken apart the way `new_obj` or `alloc_fixed` made it.
    unsafe {
        match o.kind {
            LIST if crate::flat::packed(o.ty()) => {
                let l = Box::from_raw(p as *mut ListObj);
                let cap = l.cap * crate::flat::item_size(o.ty()) / 8;
                drop(Vec::from_raw_parts(l.ptr, cap, cap));
            }
            LIST => {
                let width = crate::flat::width(o.ty()).max(1);
                let l = Box::from_raw(p as *mut ListObj);
                drop(Vec::from_raw_parts(l.ptr, l.len * width, l.cap * width));
            }
            FIXED if (o.fixed_len as usize) <= crate::pool::MAX_SLOTS => {
                crate::pool::give(p as *mut u8, o.fixed_len as usize)
            }
            FIXED => std::alloc::dealloc(p as *mut u8, fixed_layout(o.fixed_len as usize)),
            _ => {
                std::ptr::drop_in_place(p as *mut WithData);
                crate::pool::give(p as *mut u8, DATA_CLASS);
            }
        }
    }
}

/// Asked first when an object's last reference goes: `true` when it isn't
/// freed now (a value waiting for its `drop`, `rt_drops.rs`). Set only by
/// programs with `Drop`, so the others don't link that code.
static DROP_HOOK: std::sync::OnceLock<fn(i64) -> bool> = std::sync::OnceLock::new();

pub(crate) fn set_drop_hook(hook: fn(i64) -> bool) {
    let _ = DROP_HOOK.set(hook);
}

fn waits_for_drop(p: i64) -> bool {
    DROP_HOOK.get().is_some_and(|hook| hook(p))
}

pub fn retain(p: i64) {
    if p != 0 {
        obj_mut(p).rc += 1;
    }
}

/// Frees `p` and releases what it holds: called when its count reaches 0
/// (by generated code too, which counts references itself).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_free(p: i64) {
    if waits_for_drop(p) {
        return;
    }
    let o = obj(p);
    match o.kind {
        LIST | FIXED if !holds_refs(o.ty()) => {}
        // A list of text holds objects only for its long items.
        LIST if crate::flat::is_text(o.ty()) => crate::text_items::release_all(slots(p)),
        // A list of objects that hold no references themselves (records of
        // numbers, text): each item is freed here when this was its last
        // reference, without going through `kv_free` for each one.
        LIST if matches!(shape(o.ty()), Shape::List(t) if !holds_refs(*t)) => {
            for &slot in slots(p) {
                if slot != 0 {
                    let item = obj_mut(slot as i64);
                    item.rc -= 1;
                    if item.rc == 0 && !waits_for_drop(slot as i64) {
                        free(slot as i64);
                    }
                }
            }
        }
        DATA => match &*data(p).borrow() {
            Data::Context(c) => release(c.1 as i64),
            Data::Json(_, _, made) => release(*made),
            // Cells that copies share (`send.rs`): what they hold goes
            // with the last copy.
            Data::Task(_) | Data::Channel(_) | Data::Shared(_) => crate::send::free_cell(p),
            Data::Map(t) => {
                for (i, slot) in t.slots.iter().enumerate() {
                    release_slot(*slot as i64, slot_type(p, i));
                }
            }
            _ => {}
        },
        // A list of objects: every item has the list's item type.
        LIST if crate::flat::width(o.ty()) == 0
            && let Shape::List(t) = shape(o.ty()) =>
        {
            for &slot in slots(p) {
                release_slot(slot as i64, *t);
            }
        }
        _ => {
            for (i, slot) in slots(p).iter().enumerate() {
                release_slot(*slot as i64, slot_type(p, i));
            }
        }
    }
    free(p);
}

pub fn release(p: i64) {
    if p == 0 {
        return;
    }
    let o = obj_mut(p);
    o.rc -= 1;
    if o.rc == 0 {
        kv_free(p);
    }
}

pub fn release_slot(slot: i64, t: Type) {
    if t.is_ref() {
        release(slot);
    }
}

/// An object that only the caller holds, ready to change: `p` itself when
/// nobody else has it, otherwise a copy (copy-on-write).
pub fn unique(p: i64) -> i64 {
    let o = obj(p);
    if o.rc == 1 {
        return p;
    }
    // A packed list's buffer copies as it is (decisions 0030 and 0045).
    if o.kind == LIST && crate::flat::packed(o.ty()) {
        let copy = new_list_raw(o.ty(), o.tag, slots(p).to_vec(), list_mut(p).len);
        release(p);
        return copy;
    }
    let data = data(p).borrow().clone();
    let copied = slots(p).to_vec();
    let held = match &data {
        Data::Map(t) => t.slots.clone(),
        Data::Context(c) => vec![c.1],
        Data::Json(_, _, made) => vec![*made as u64],
        _ if crate::flat::is_text(o.ty()) => {
            crate::text_items::retain_all(&copied);
            Vec::new()
        }
        _ => copied.clone(),
    };
    for (i, slot) in held.iter().enumerate() {
        let t = match &data {
            Data::Context(..) => o.ty(),
            Data::Json(..) => slot_type(p, 0),
            _ => slot_type(p, i),
        };
        if t.is_ref() {
            retain(*slot as i64);
        }
    }
    let copy = new_obj(o.ty(), o.tag, copied, data);
    release(p);
    copy
}
