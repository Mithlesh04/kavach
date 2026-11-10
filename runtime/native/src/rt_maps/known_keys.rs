//! `map["key"]` for a constant key, as generated code calls it.
//!
//! A constant key's address names its text for the whole run, so a slot
//! kept by address remembers its hash and where it was found last. Read
//! from JSON objects of one shape, the key is at the same entry again, and
//! it is the same object each time: the parser makes one object for each
//! key's text. Once the key is found as the same object twice running, the
//! slot holds that object, so it can't be freed and its address can't
//! name anything else: an entry whose key is that object is this key, with
//! no hash or text to read. Python finds keys the same way, by comparing
//! the addresses of keys it keeps one copy of.

use crate::heap::{Data, release, retain};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many constant keys are kept: a slot each, by address.
const KEYS_SEEN: usize = 64;

/// What a slot knows of one constant key.
#[derive(Clone, Copy)]
struct Known {
    /// The key's address and length.
    ptr: usize,
    len: usize,
    hash: u64,
    /// The entry it was found at last.
    hint: usize,
    /// The key object it was found as last.
    key: u64,
    /// `key` is held here: found as the same object twice running.
    held: bool,
}

const NOTHING: Known = Known {
    ptr: 0,
    len: 0,
    hash: 0,
    hint: usize::MAX,
    key: 0,
    held: false,
};

thread_local! {
    static SEEN: [Cell<Known>; KEYS_SEEN] = const { [const { Cell::new(NOTHING) }; KEYS_SEEN] };
}

/// Key objects the slots hold, on every thread: the leak check doesn't
/// count them (`lib.rs`).
static HELD: AtomicUsize = AtomicUsize::new(0);

/// How many key objects the slots of every thread still hold.
pub(crate) fn held() -> usize {
    HELD.load(Ordering::Relaxed)
}

/// The slot of the constant key at `ptr`. Constants lie byte after byte
/// (`"qty"`, then `"price"`), so the whole address is mixed: its low bits
/// alone would give neighbouring keys one slot, each pushing the other
/// out, and every lookup would hash its key again and probe from the start.
fn seen_slot(ptr: *const u8) -> usize {
    let mixed = (ptr as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    (mixed >> (64 - KEYS_SEEN.trailing_zeros())) as usize
}

/// `map[key]` for a text key and a value held by reference, with `map`
/// borrowed and the key given as UTF-8 bytes: the value (retained), or 0
/// for `none`. What generated code calls for `j["name"]`: no text object
/// is made for the key, and nothing is dispatched on the map's type.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_map_get_text(map: i64, ptr: *const u8, len: usize) -> i64 {
    let at = seen_slot(ptr);
    let was = SEEN.with(|seen| seen[at].get());
    let mine = was.ptr == ptr as usize && was.len == len;
    let found = match &*crate::heap::data(map).borrow() {
        Data::Map(t) => {
            if mine && was.held {
                let e = if t.slots.get(was.hint * 2) == Some(&was.key) {
                    Some(was.hint)
                } else {
                    held_entry(t, was.key)
                };
                if let Some(e) = e {
                    let value = t.slots[e * 2 + 1] as i64;
                    retain(value);
                    if e != was.hint {
                        SEEN.with(|seen| seen[at].set(Known { hint: e, ..was }));
                    }
                    return value;
                }
            }
            let key = crate::rt::text(ptr, len);
            let (hash, hint) = if mine {
                (was.hash, was.hint)
            } else {
                (super::text_hash(key), usize::MAX)
            };
            let same = |k: u64| crate::heap::with_str(k as i64, |s| s == key);
            t.find_from(hint, hash, same).map(|e| {
                let value = t.slots[e * 2 + 1] as i64;
                retain(value);
                (value, e, t.slots[e * 2], hash)
            })
        }
        _ => None,
    };
    let Some((value, hint, key, hash)) = found else {
        return 0;
    };
    // `key` is a live object with this text, so it may be held.
    let same_object = mine && was.key == key;
    let held = same_object && (was.held || hold(key));
    if was.held && !same_object {
        HELD.fetch_sub(1, Ordering::Relaxed);
        release(was.key as i64);
    }
    let ptr = ptr as usize;
    let now = Known {
        ptr,
        len,
        hash,
        hint,
        key,
        held,
    };
    SEEN.with(|seen| seen[at].set(now));
    value
}

/// Maps this small are scanned for a held key object: objects of one
/// shape whose fields differ (`note` there in some, not in others) have
/// the key at another entry, and their keys fill a cache line or two.
const SCAN: usize = 8;

/// The entry of small table `t` whose key is held object `key`.
fn held_entry(t: &crate::table::Table, key: u64) -> Option<usize> {
    if t.slots.len() > 2 * SCAN {
        return None;
    }
    t.slots.iter().step_by(2).position(|&k| k == key)
}

/// Holds key object `key` in a slot; `true`.
fn hold(key: u64) -> bool {
    static AT_EXIT: std::sync::Once = std::sync::Once::new();
    AT_EXIT.call_once(|| crate::rt::at_exit(forget_held));
    retain(key as i64);
    HELD.fetch_add(1, Ordering::Relaxed);
    true
}

/// Lets go of the key objects this thread's slots hold: the program ends.
fn forget_held() {
    SEEN.with(|seen| {
        for slot in seen {
            let was = slot.replace(NOTHING);
            if was.held {
                HELD.fetch_sub(1, Ordering::Relaxed);
                release(was.key as i64);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    /// Constant keys lie byte after byte, so neighbours must not share a
    /// slot of the key cache: then each would push the other out.
    #[test]
    fn neighbouring_constant_keys_get_slots_of_their_own() {
        for base in [0x5555_0000usize, 0x7ff6_1234_5671, 0x40_1003] {
            for gap in 1..=20 {
                let (a, b) = (base as *const u8, (base + gap) as *const u8);
                assert_ne!(
                    super::seen_slot(a),
                    super::seen_slot(b),
                    "{base:#x} + {gap}"
                );
            }
        }
    }
}
