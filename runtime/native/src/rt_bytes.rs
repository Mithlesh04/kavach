//! `Bytes` in native code (spec 2.13), as in the interpreter
//! (`compiler/interp/src/bytes.rs`): raw data kept in one buffer, one
//! byte per byte.

use crate::heap::{Data, alloc, alloc_slots, release};
use crate::rt::{panic_with, type_of};
use crate::shapes::{first_of, shape, with_str};
use kavach_kir::{RtOp, Shape, Type};

/// A new `Bytes` object of type `ty`.
pub fn new_bytes(b: Vec<u8>, ty: Type) -> i64 {
    alloc(ty, Data::Bytes(b))
}

/// The bytes of `p` (borrowed).
pub fn with_bytes<R>(p: i64, f: impl FnOnce(&[u8]) -> R) -> R {
    match &*crate::heap::data(p).borrow() {
        Data::Bytes(b) => f(b),
        _ => f(&[]),
    }
}

/// The `Bytes` type of the program (for `"…".bytes()`).
fn bytes_type() -> Type {
    first_of(&Shape::Bytes)
}

/// The `Bytes` ops (operands owned). `code` is the result type.
pub fn bytes_op(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    let result = type_of(code);
    let out = match op {
        RtOp::BytesNew => {
            // Straight off the list's own buffer: packed lists
            // (decision 0030) already hold one byte per byte.
            let data = if crate::flat::packed(crate::heap::obj(a).ty()) {
                // Whole words at a time: the packed order is the byte
                // order (little-endian), so this is a straight copy.
                let words = crate::heap::slots(a);
                let n = crate::heap::list_len_items_pub(a);
                let mut data = Vec::with_capacity(n.next_multiple_of(8));
                for w in &words[..n.div_ceil(8)] {
                    data.extend_from_slice(&w.to_le_bytes());
                }
                data.truncate(n);
                data
            } else {
                crate::heap::slots(a).iter().map(|s| *s as u8).collect()
            };
            new_bytes(data, result)
        }
        RtOp::StrBytes => new_bytes(with_str(a, |s| s.as_bytes().to_vec()), bytes_type()),
        RtOp::BytesAt => {
            let at = with_bytes(a, |d| {
                usize::try_from(b).ok().and_then(|i| d.get(i).copied())
            });
            match at {
                Some(x) => x as i64,
                None => {
                    let n = with_bytes(a, <[u8]>::len);
                    panic_with(
                        &kavach_runtime_core::bytes_index_panic(i128::from(b), n),
                        line,
                    )
                }
            }
        }
        RtOp::BytesLen => with_bytes(a, <[u8]>::len) as i64,
        RtOp::BytesGet => {
            let at = with_bytes(a, |d| {
                usize::try_from(b).ok().and_then(|i| d.get(i).copied())
            });
            crate::rt_data::optional(at.map(|x| x as i64), code)
        }
        RtOp::BytesSlice => {
            let data = with_bytes(a, |d| {
                let clamp = |n: i64| n.clamp(0, d.len() as i64) as usize;
                let (start, end) = (clamp(b), clamp(c));
                d[start..end.max(start)].to_vec()
            });
            new_bytes(data, result)
        }
        RtOp::BytesText => {
            let text = with_bytes(a, |d| std::str::from_utf8(d).ok().map(str::to_string));
            match (text, shape(result)) {
                (Some(t), Shape::Optional(inner)) => crate::shapes::new_text(&t, *inner),
                _ => 0,
            }
        }
        RtOp::BytesCrc32c => {
            let crc = with_bytes(a, crc32c_fast);
            crate::heap::release(a);
            return crc as i64;
        }
        RtOp::BytesXxhash64 => {
            let h = with_bytes(a, kavach_runtime_core::files::xxhash64);
            crate::heap::release(a);
            return h as i64;
        }
        RtOp::BytesToList => {
            let slots = with_bytes(a, |d| d.iter().map(|x| *x as u64).collect());
            alloc_slots(result, 0, slots)
        }
        RtOp::BytesAsList => with_bytes(a, |d| as_list(d, result)),
        _ => {
            // BytesConcat: `a + b`.
            let data = with_bytes(a, |x| with_bytes(b, |y| [x, y].concat()));
            release(b);
            new_bytes(data, result)
        }
    };
    release(a);
    out
}

/// `b.as_list<T>()` (spec 2.13): `d` as the list inside optional type
/// `result`, or 0 (`none`) when it isn't a whole number of items. Lists of
/// small numbers are packed little-endian (decision 0045) and slots hold
/// the bits, so whole words copy straight across.
fn as_list(d: &[u8], result: Type) -> i64 {
    let Shape::Optional(list) = shape(result) else {
        return 0;
    };
    let Shape::List(item) = shape(*list) else {
        return 0;
    };
    let packed = kavach_kir::packed_size(*item);
    let size = if packed == 0 { 8 } else { packed };
    if !d.len().is_multiple_of(size) {
        return 0;
    }
    if d.is_empty() {
        return alloc_slots(*list, 0, Vec::new());
    }
    // Whole words first: a loop the compiler turns into one bulk copy.
    let (whole, rest) = d.as_chunks::<8>();
    let mut words = Vec::with_capacity(d.len().div_ceil(8));
    words.extend(whole.iter().map(|c| u64::from_le_bytes(*c)));
    if !rest.is_empty() {
        let mut word = [0; 8];
        word[..rest.len()].copy_from_slice(rest);
        words.push(u64::from_le_bytes(word));
    }
    if packed == 0 {
        return alloc_slots(*list, 0, words);
    }
    crate::heap::new_list_raw(*list, 0, words, d.len() / size)
}

/// CRC32C by the processor where it can be (SSE 4.2), the shared
/// software tables otherwise.
fn crc32c_fast(data: &[u8]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    if std::arch::is_x86_feature_detected!("sse4.2") {
        // SAFETY: the check above proves the processor has these
        // instructions.
        return unsafe { crc32c_sse(data) };
    }
    kavach_runtime_core::files::crc32c(data)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
#[allow(unsafe_code)]
unsafe fn crc32c_sse(data: &[u8]) -> u32 {
    use std::arch::x86_64::{_mm_crc32_u8, _mm_crc32_u64};
    let mut crc = !0u32 as u64;
    let (chunks, rest) = data.as_chunks::<8>();
    for c in chunks {
        crc = _mm_crc32_u64(crc, u64::from_le_bytes(*c));
    }
    let mut crc = crc as u32;
    for &b in rest {
        crc = _mm_crc32_u8(crc, b);
    }
    !crc
}
