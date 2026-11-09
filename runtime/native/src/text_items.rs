//! Items of a list of text (`docs/architecture/flat-lists.md`): two words
//! each. Text up to 15 bytes is stored in them; its last byte is a tag
//! (`INLINE`, the `ASCII` flag and the length). Longer text is a pointer
//! to a text object in the first word, with 0 in the second.
//!
//! So splitting text into short words makes no objects at all, and the
//! length of an ASCII word is read from its tag.

use crate::heap::{release, retain};
use crate::shapes::{new_text, with_str};
use kavach_kir::Type;

/// Words per item.
pub const WIDTH: usize = 2;

/// The longest text stored in an item.
pub const SHORT: usize = 15;

/// Bits of the tag (the item's last byte, bits 56–63 of its second word).
pub const INLINE: u8 = 0x80;
pub const ASCII: u8 = 0x40;
pub const LEN_MASK: u8 = 0x0f;

/// The item holding `s`: inline when short, otherwise a new text object
/// of type `ty`.
pub fn encode(s: &str, ty: Type) -> [u64; 2] {
    if s.len() > SHORT {
        return [new_text(s, ty) as u64, 0];
    }
    let [low, high] = pack(s.as_bytes());
    // ASCII when no byte has its top bit set: one test on both words.
    let ascii = (low | high) & 0x8080_8080_8080_8080 == 0;
    let tag = INLINE | if ascii { ASCII } else { 0 } | s.len() as u8;
    [low, high | (tag as u64) << 56]
}

/// The item holding the text object `p`, consuming its reference: copied
/// in when short (and `p` released), otherwise `p` itself.
pub fn encode_object(p: i64) -> [u64; 2] {
    let short = with_str(p, |s| (s.len() <= SHORT).then(|| encode(s, Type::I64)));
    match short {
        Some(item) => {
            release(p);
            item
        }
        None => [p as u64, 0],
    }
}

/// Up to 15 bytes as two little-endian words, the rest 0. Overlapping
/// loads of 8 or 4 bytes instead of a byte-by-byte copy.
fn pack(b: &[u8]) -> [u64; 2] {
    let n = b.len();
    let word = |at: usize| u64::from_le_bytes(b[at..at + 8].try_into().unwrap_or_default());
    let half = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap_or_default()) as u64;
    match n {
        // The last 8 bytes, moved down so that byte 8 comes first.
        8.. => [
            word(0),
            word(n - 8).checked_shr(8 * (16 - n) as u32).unwrap_or(0),
        ],
        // The first and last 4 bytes overlap; the shared bytes agree.
        4.. => [half(0) | half(n - 4) << (8 * (n - 4)), 0],
        _ => [b.iter().rev().fold(0, |w, c| w << 8 | *c as u64), 0],
    }
}

/// Calls `f` with the text of `item` (borrowed).
pub fn with_item<R>(item: &[u64], f: impl FnOnce(&str) -> R) -> R {
    let tag = (item[1] >> 56) as u8;
    if tag & INLINE == 0 {
        return with_str(item[0] as i64, f);
    }
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&item[0].to_le_bytes());
    bytes[8..].copy_from_slice(&item[1].to_le_bytes());
    let len = (tag & LEN_MASK) as usize;
    // SAFETY: inline bytes are only ever copied from a whole `&str`
    // (`encode`), so they are valid UTF-8.
    #[allow(unsafe_code)]
    f(unsafe { std::str::from_utf8_unchecked(&bytes[..len]) })
}

/// `item` as an owned text object of type `ty`: a new one when it is
/// inline, otherwise its object, retained.
pub fn boxed(item: &[u64], ty: Type) -> i64 {
    if (item[1] >> 56) as u8 & INLINE == 0 {
        retain(item[0] as i64);
        return item[0] as i64;
    }
    with_item(item, |s| new_text(s, ty))
}

/// Releases the objects behind the long items in `words`.
pub fn release_all(words: &[u64]) {
    for item in words.chunks(WIDTH) {
        if (item[1] >> 56) as u8 & INLINE == 0 {
            release(item[0] as i64);
        }
    }
}

/// Retains the objects behind the long items in `words` (a copy of a list).
pub fn retain_all(words: &[u64]) {
    for item in words.chunks(WIDTH) {
        if (item[1] >> 56) as u8 & INLINE == 0 {
            retain(item[0] as i64);
        }
    }
}

/// The items of `words`, joined with `sep` (`list.join`). The length is
/// added up first, so the text is written into one buffer with no checks:
/// a short item's 16 bytes at once, then the end moves past its text.
#[allow(unsafe_code)]
pub fn join(words: &[u64], sep: &str) -> crate::text::Text {
    let len = |item: &[u64]| {
        let tag = (item[1] >> 56) as u8;
        if tag & INLINE != 0 {
            (tag & LEN_MASK) as usize
        } else {
            with_str(item[0] as i64, str::len)
        }
    };
    let n = words.len() / WIDTH;
    let total = sep.len() * n.saturating_sub(1) + words.chunks(WIDTH).map(len).sum::<usize>();
    let mut out: Vec<u8> = Vec::with_capacity(total + 16);
    let (dst, mut at) = (out.as_mut_ptr(), 0);
    for (i, item) in words.chunks(WIDTH).enumerate() {
        // SAFETY: the separators and items written add up to `total`
        // bytes, and a short item's 16 bytes start at most `total - len`:
        // all inside the `total + 16` bytes of `out`. Only the text bytes
        // count toward `at`.
        unsafe {
            match sep.as_bytes() {
                _ if i == 0 => {}
                &[byte] => {
                    *dst.add(at) = byte;
                    at += 1;
                }
                bytes => {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst.add(at), bytes.len());
                    at += bytes.len();
                }
            }
            let tag = (item[1] >> 56) as u8;
            if tag & INLINE == 0 {
                with_str(item[0] as i64, |s| {
                    std::ptr::copy_nonoverlapping(s.as_ptr(), dst.add(at), s.len());
                    at += s.len();
                });
                continue;
            }
            (dst.add(at) as *mut [u64; 2]).write_unaligned([item[0], item[1]]);
            at += (tag & LEN_MASK) as usize;
        }
    }
    // SAFETY: the first `total` bytes are written (`at == total`), and
    // they are whole texts and separators, all valid UTF-8.
    unsafe {
        out.set_len(at);
        String::from_utf8_unchecked(out)
    }
    .into()
}

/// The items of `s` split at the ASCII byte `sep` (`text.split`), about
/// `n` of them. Eight bytes are searched at once, and a short part's item
/// is two masked 8-byte loads instead of a byte-by-byte copy.
pub fn split_at_byte(s: &str, sep: u8, n: usize, ty: Type) -> Vec<u64> {
    const LOW7: u64 = 0x7f7f_7f7f_7f7f_7f7f;
    let b = s.as_bytes();
    let pattern = u64::from_le_bytes([sep; 8]);
    let mut words = Vec::with_capacity(n * WIDTH);
    let (mut start, mut i) = (0, 0);
    while i + 8 <= b.len() {
        let w = word(b, i) ^ pattern;
        // The top bit of each byte of `w` that is 0 (a `sep`), exactly:
        // adding 0x7f to the low 7 bits never carries into the next byte.
        let mut hits = !((w & LOW7).wrapping_add(LOW7) | w | LOW7);
        while hits != 0 {
            let at = i + hits.trailing_zeros() as usize / 8;
            push_part(&mut words, s, start, at, ty);
            start = at + 1;
            hits &= hits - 1;
        }
        i += 8;
    }
    for (at, byte) in b.iter().enumerate().skip(i) {
        if *byte == sep {
            push_part(&mut words, s, start, at, ty);
            start = at + 1;
        }
    }
    push_part(&mut words, s, start, b.len(), ty);
    words
}

/// Adds the item for `s[start..end]`: from two loads when it is short
/// and 16 bytes from `start` are inside `s`, otherwise by `encode`.
fn push_part(words: &mut Vec<u64>, s: &str, start: usize, end: usize, ty: Type) {
    let (b, len) = (s.as_bytes(), end - start);
    if len > SHORT || start + 16 > b.len() {
        words.extend_from_slice(&encode(&s[start..end], ty));
        return;
    }
    let mask = |n: usize| if n >= 8 { u64::MAX } else { (1 << (8 * n)) - 1 };
    let low = word(b, start) & mask(len);
    let high = if len > 8 {
        word(b, start + 8) & mask(len - 8)
    } else {
        0
    };
    let ascii = (low | high) & 0x8080_8080_8080_8080 == 0;
    let tag = INLINE | if ascii { ASCII } else { 0 } | len as u8;
    words.extend_from_slice(&[low, high | (tag as u64) << 56]);
}

fn word(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap_or_default())
}

/// The number of characters of `item`: `len()` (spec 2.2).
pub fn char_len(item: &[u64]) -> i64 {
    with_item(item, |s| {
        if s.is_ascii() {
            s.len()
        } else {
            s.chars().count()
        }
    }) as i64
}

/// The item at `addr` in a list of text, as an owned text object of
/// type `code`: for generated code.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_text_box(addr: i64, code: i64) -> i64 {
    boxed(item_at(addr), crate::rt::type_of(code))
}

/// Stores text object `p` (consumed) into the empty item at `addr`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_text_store(addr: i64, p: i64) {
    write_item(addr, encode_object(p));
}

/// The number of characters of the item at `addr` (borrowed).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_text_len(addr: i64) -> i64 {
    char_len(item_at(addr))
}

/// Appends `more` to the item being built at `addr` (`"…{x}…"` going
/// straight into a list of text): it stays inline while it fits, then
/// becomes a text object of type `str_code`.
fn item_append(addr: i64, more: &str, str_code: i64) {
    let item: [u64; 2] = item_at(addr).try_into().unwrap_or_default();
    let tag = (item[1] >> 56) as u8;
    let len = (tag & LEN_MASK) as usize;
    let grown = if tag & INLINE == 0 {
        let p = crate::rt_data::format_into(item[0] as i64, str_code, |t| t.push_str(more));
        [p as u64, 0]
    } else if len + more.len() <= SHORT {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&item[0].to_le_bytes());
        bytes[8..].copy_from_slice(&item[1].to_le_bytes());
        bytes[len..len + more.len()].copy_from_slice(more.as_bytes());
        let ascii = if more.is_ascii() { tag & ASCII } else { 0 };
        bytes[15] = INLINE | ascii | (len + more.len()) as u8;
        let word = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap_or_default());
        [word(0), word(8)]
    } else {
        let text = with_item(&item, |s| {
            let mut t = crate::text::Text::with_capacity(s.len() + more.len());
            t.push_str(s);
            t.push_str(more);
            t
        });
        let ty = crate::rt::type_of(str_code);
        [
            crate::heap::alloc(ty, crate::heap::Data::Str(text)) as u64,
            0,
        ]
    };
    write_item(addr, grown);
}

/// Appends constant text to the item being built at `addr`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_item_text(addr: i64, ptr: *const u8, len: usize, str_code: i64) {
    item_append(addr, crate::rt::text(ptr, len), str_code);
}

/// Appends value `slot` (consumed), shown as text, to the item being
/// built at `addr`. Short values are shown into the stack, not the heap.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_item_format(addr: i64, slot: i64, code: i64, str_code: i64) {
    let ty = crate::rt::type_of(code);
    // A whole number, the usual case (`"word{i}"`): its digits go straight
    // into the item.
    if ty.is_int() {
        let negative = ty.is_signed() && slot < 0;
        let n = if ty.is_signed() {
            slot.unsigned_abs()
        } else {
            slot as u64
        };
        item_append(
            addr,
            crate::display::int_text(&mut [0; 21], n, negative),
            str_code,
        );
        return;
    }
    let mut shown = crate::text::Text::with_capacity(0);
    crate::display::append_display(&mut shown, slot, ty);
    crate::heap::release_slot(slot, ty);
    item_append(addr, shown.as_str(), str_code);
}

/// The item built at `addr` as an owned text object of type `code`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_item_take(addr: i64, code: i64) -> i64 {
    let item = item_at(addr);
    if (item[1] >> 56) as u8 & INLINE == 0 {
        return item[0] as i64;
    }
    with_item(item, |s| new_text(s, crate::rt::type_of(code)))
}

fn write_item(addr: i64, item: [u64; 2]) {
    // SAFETY: generated code passes the address of an item: two words in
    // a list of text with room for it, or on the stack.
    #[allow(unsafe_code)]
    unsafe {
        (addr as *mut [u64; 2]).write(item);
    }
}

#[allow(unsafe_code)]
fn item_at<'a>(addr: i64) -> &'a [u64] {
    // SAFETY: generated code passes the address of a live item of a list
    // of text: two words inside its buffer, which the list keeps alive.
    unsafe { std::slice::from_raw_parts(addr as *const u64, WIDTH) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_round_trips() {
        for s in [
            "",
            "a",
            "ab",
            "abc",
            "abcd",
            "abcde",
            "word42",
            "seven77",
            "eight888",
            "nine99999",
            "fifteen bytes!!",
            "नम",
            "ünï",
        ] {
            let item = encode(s, Type::I64);
            assert_eq!(with_item(&item, str::to_string), s);
            assert_eq!(char_len(&item), s.chars().count() as i64);
            let tag = (item[1] >> 56) as u8;
            assert_eq!(tag & ASCII != 0, s.is_ascii(), "{s}");
        }
    }

    #[test]
    fn splitting_at_a_byte_matches_encoding_each_part() {
        let text = "a bb  ccc dddddddd eeeeeeeeeeeeeee ffffffffffffffff ü gü€ é";
        for len in 0..=text.len() {
            let Some(s) = text.get(..len) else { continue };
            let want: Vec<u64> = s.split(' ').flat_map(|p| encode(p, Type::I64)).collect();
            let got = split_at_byte(s, b' ', s.split(' ').count(), Type::I64);
            assert_eq!(got.len(), want.len());
            // Short items are equal word for word; long ones are objects
            // holding the same text.
            for (g, w) in got.chunks(WIDTH).zip(want.chunks(WIDTH)) {
                assert_eq!(with_item(g, str::to_string), with_item(w, str::to_string));
                assert!(g == w || g[1] == 0, "{s:?}");
            }
        }
    }

    #[test]
    fn appending_to_an_item_matches_encoding_the_whole_text() {
        for parts in [
            &["word", "42"][..],
            &["", "ü", "x"],
            &["abc", "", "defghijklmno"],
        ] {
            let mut item = encode("", Type::I64);
            let addr = item.as_mut_ptr() as i64;
            for p in parts {
                item_append(addr, p, 0);
            }
            assert_eq!(item, encode(&parts.concat(), Type::I64));
        }
    }
}
