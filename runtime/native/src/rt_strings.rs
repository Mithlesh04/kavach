//! Text in native code: making it, printing values into it (string
//! interpolation), its length, comparing and joining it (`+`, which grows
//! text with one holder in place, spec 6.1). Split from `rt_data.rs`,
//! which re-exports these.

use crate::display::{append_display, compare, display};
use crate::heap::{Data, new_str, obj, release, release_slot, str_of, unique, with_str};
use crate::rt::{text, type_of};

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_str_new(ptr: *const u8, len: usize, code: i64) -> i64 {
    crate::heap::new_text(text(ptr, len), type_of(code))
}

/// Appends the printed form of a value to the text `acc` (0 starts new text).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_format(acc: i64, slot: i64, code: i64, str_code: i64) -> i64 {
    let ty = type_of(code);
    let acc = format_into(acc, str_code, |s| append_display(s, slot, ty));
    release_slot(slot, ty);
    acc
}

/// The number of characters of text `p` (borrowed): `len()` (spec 2.2).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_str_len(p: i64) -> i64 {
    with_str(p, |s| {
        if s.is_ascii() {
            s.len()
        } else {
            s.chars().count()
        }
    }) as i64
}

/// Appends constant text to the text `acc` (0 starts new text).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_format_text(
    acc: i64,
    ptr: *const u8,
    len: usize,
    str_code: i64,
) -> i64 {
    format_into(acc, str_code, |s| s.push_str(text(ptr, len)))
}

/// Text being built by string interpolation: `acc` (0 starts new text),
/// with `add` appending to it. The text is only ever held here, so it
/// grows in place.
pub(crate) fn format_into(
    acc: i64,
    str_code: i64,
    add: impl FnOnce(&mut crate::text::Text),
) -> i64 {
    let acc = if acc == 0 {
        crate::heap::alloc(
            type_of(str_code),
            Data::Str(crate::text::Text::with_capacity(0)),
        )
    } else {
        unique(acc)
    };
    if let Data::Str(s) = &mut *crate::heap::data(acc).borrow_mut() {
        add(s);
    }
    acc
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_print_value(slot: i64, code: i64) {
    let ty = type_of(code);
    crate::rt::print_piece(&display(slot, ty, true));
    release_slot(slot, ty);
}

/// -1, 0 or 1, comparing two texts.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_cmp(a: i64, b: i64) -> i64 {
    let order = compare(a, b) as i64;
    release(a);
    release(b);
    order
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_concat(a: i64, b: i64) -> i64 {
    // Text with no other holder grows in place (spec 6.1), so `s += part`
    // in a loop copies each part once, not the whole text each time.
    if obj(a).rc == 1 && a != b {
        if let Data::Str(s) = &mut *crate::heap::data(a).borrow_mut() {
            with_str(b, |more| s.push_str(more));
        }
        release(b);
        return a;
    }
    let ty = obj(a).ty();
    let joined = str_of(a) + &str_of(b);
    release(a);
    release(b);
    new_str(joined, ty)
}
