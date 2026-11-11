//! Run-time functions for heap values. Every function that takes an
//! object pointer consumes the caller's reference unless its name ends in
//! `_raw` (borrowed). Results are owned by the caller.

use crate::display::{append_display, boxed, compare, display, equal};
use crate::heap::{
    Data, alloc, innermost, items_of, len_of, new_str, obj, release, release_slot, retain, shape,
    slot_at, str_of, unique, with_str,
};
use crate::rt::{panic_with, text, type_of};
use kavach_kir::{RtOp, Shape, Type};

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_retain(p: i64) {
    retain(p);
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_release(p: i64) {
    release(p);
}

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

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_eq(a: i64, b: i64, code: i64) -> i8 {
    let ty = type_of(code);
    let same = equal(a, b, ty);
    release_slot(a, ty);
    release_slot(b, ty);
    same as i8
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
    let ty = obj(a).ty();
    let joined = str_of(a) + &str_of(b);
    release(a);
    release(b);
    new_str(joined, ty)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_record(code: i64, variant: i64, n: i64) -> i64 {
    crate::heap::alloc_zeroed(type_of(code), variant as u32, n as usize)
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_list(code: i64, n: i64) -> i64 {
    crate::heap::alloc_zeroed(type_of(code), 0, n as usize)
}

/// Fills slot `i` of a new object (the slot's reference moves in). In a
/// flat list, item `i` takes the fields (or text) of the object `slot`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_init(p: i64, i: i64, slot: i64) {
    if let size @ 1.. = crate::flat::item_size(obj(p).ty()) {
        return crate::flat::set_item(crate::heap::slots_mut(p), i as usize, size, slot as u64);
    }
    match crate::flat::width(obj(p).ty()) {
        0 => crate::heap::slots_mut(p)[i as usize] = slot as u64,
        w => {
            let at = i as usize * w;
            crate::flat::store(&mut crate::heap::slots_mut(p)[at..at + w], slot);
        }
    }
}

/// Field or element `i`, as an owned value. An error wrapped by `context`
/// is read through the wrapper.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_get(p: i64, i: i64) -> i64 {
    let slot = crate::shapes::item_owned(innermost(p), i as usize);
    release(p);
    slot
}

/// The variant number of union value `p` (borrowed), looking inside
/// `context` wrappers: what generated code calls when `p` is not a fixed
/// object.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_variant_of(p: i64) -> i64 {
    crate::heap::tag_of(innermost(p)) as i64
}

/// Field `i` of union value `p` (borrowed) as an owned slot, like
/// `kv_variant_of`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_field_of(p: i64, i: i64) -> i64 {
    crate::shapes::item_owned(innermost(p), i as usize)
}

/// A copy of `p` with slot `i` replaced (copy-on-write).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_set(p: i64, i: i64, slot: i64) -> i64 {
    let q = unique(p);
    let ty = slot_at(q, i as usize).1;
    let old = std::mem::replace(&mut crate::heap::slots_mut(q)[i as usize], slot as u64);
    release_slot(old as i64, ty);
    q
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_has_variant(p: i64, i: i64) -> i8 {
    let yes = crate::heap::tag_of(innermost(p)) as i64 == i;
    release(p);
    yes as i8
}

/// `list[i]` or element `i` of a tuple; out of range is a panic (spec 5.6).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_index(p: i64, i: i64, line: i32) -> i64 {
    let n = len_of(p) as i64;
    if i < 0 || i >= n {
        kv_bad_index(i, n, line)
    }
    kv_get(p, i)
}

/// Index `i` is out of range of a list with `n` items: a panic. Generated
/// code calls it on its rare path, holding no reference for it to drop.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_bad_index(i: i64, n: i64, line: i32) -> ! {
    panic_with(
        &kavach_runtime_core::list_index_panic(i128::from(i), n as usize),
        line,
    )
}

/// Wraps a scalar as a present optional of type `code`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_some(slot: i64, code: i64) -> i64 {
    alloc(type_of(code), Data::Boxed(slot as u64))
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_is_some(p: i64) -> i8 {
    release(p);
    (p != 0) as i8
}

/// The value inside an optional of type `code`; `none` is a panic.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_unwrap(p: i64, code: i64, line: i32) -> i64 {
    if p == 0 {
        panic_with("`.unwrap()` on `none`", line)
    }
    match shape(type_of(code)) {
        Shape::Optional(inner) if !inner.is_ref() => {
            let v = boxed(p);
            release(p);
            v
        }
        _ => p,
    }
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_len_raw(p: i64) -> i64 {
    len_of(p) as i64
}

#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_item_raw(p: i64, i: i64) -> i64 {
    crate::shapes::item_owned(p, i as usize)
}

/// An optional of type `code` holding `slot` (already owned), or `none`.
pub fn optional(slot: Option<i64>, code: i64) -> i64 {
    match (slot, shape(type_of(code))) {
        (None, _) => 0,
        (Some(s), Shape::Optional(inner)) if !inner.is_ref() => kv_some(s, code),
        (Some(s), _) => s,
    }
}

/// Built-in methods of text, lists, maps, `Shared` and integers (spec
/// 2.2), on up to three operands. `code` is the result type.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_rt(op: i64, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    let op = RtOp::ALL[op as usize];
    let result = type_of(code);
    let flag = |x: bool| x as i64;
    // Reads the text in `p` without copying it, then releases `p`.
    fn read<R>(p: i64, f: impl FnOnce(&str) -> R) -> R {
        let r = with_str(p, f);
        release(p);
        r
    }
    match op {
        RtOp::IntPow if result.is_ref() => crate::rt_bigint::pow(a, b, code, line),
        RtOp::FloatNear => {
            let (x, y) = (f64::from_bits(a as u64), f64::from_bits(b as u64));
            match c {
                0 => i64::from(kavach_runtime_core::float_near(x, y)),
                _ => i64::from(kavach_runtime_core::float32_near(x as f32, y as f32)),
            }
        }
        RtOp::IntPow => crate::rt::int_pow(a, b, result, line),
        RtOp::MapWithCapacity => {
            let table = crate::table::Table::with_room(a.max(0) as usize);
            crate::heap::alloc(result, crate::heap::Data::Map(Box::new(table)))
        }
        RtOp::PanicText => {
            let message = with_str(a, str::to_string);
            panic_with(&message, line)
        }
        RtOp::StrLen => read(a, |s| s.chars().count()) as i64,
        RtOp::StrIsEmpty => flag(read(a, str::is_empty)),
        RtOp::StrUpper => new_str(read(a, str::to_uppercase), result),
        RtOp::StrLower => new_str(read(a, str::to_lowercase), result),
        RtOp::StrTrim => new_str(read(a, |s| s.trim().to_string()), result),
        RtOp::StrContains | RtOp::StrStartsWith | RtOp::StrEndsWith => {
            let yes = read(a, |s| {
                read(b, |t| match op {
                    RtOp::StrContains => s.contains(t),
                    RtOp::StrStartsWith => s.starts_with(t),
                    _ => s.ends_with(t),
                })
            });
            flag(yes)
        }
        RtOp::StrRepeat => {
            let n = usize::try_from(b).unwrap_or(0);
            match read(a, |s| kavach_runtime_core::repeat(s, n)) {
                Ok(text) => new_str(text, result),
                Err(message) => panic_with(&message, line),
            }
        }
        // Only a hint; native code built with Cranelift calls here.
        RtOp::ListPrefetch => {
            release(a);
            0
        }
        RtOp::ListLen | RtOp::ListIsEmpty => {
            let n = len_of(a);
            release(a);
            if op == RtOp::ListLen {
                n as i64
            } else {
                flag(n == 0)
            }
        }
        RtOp::ListPush => {
            let q = unique(a);
            crate::heap::push_item(q, b as u64);
            q
        }
        RtOp::ListGet | RtOp::ListFirst | RtOp::ListLast => {
            let n = len_of(a) as i64;
            let index = match op {
                RtOp::ListGet => b,
                RtOp::ListFirst => 0,
                _ => n - 1,
            };
            let item = (0..n).contains(&index).then(|| kv_item_raw(a, index));
            release(a);
            optional(item, code)
        }
        RtOp::WrappingAdd
        | RtOp::WrappingSub
        | RtOp::WrappingMul
        | RtOp::CheckedAdd
        | RtOp::CheckedSub
        | RtOp::CheckedMul
        | RtOp::CheckedDiv => crate::rt_numbers::int_method(op, a, b, code),
        RtOp::ListSet => crate::rt_maps::list_set(a, b, c, line),
        RtOp::StrReplace
        | RtOp::StrLines
        | RtOp::StrSplit
        | RtOp::StrChars
        | RtOp::StrIndexOf
        | RtOp::StrSlice => crate::rt_text::text_op(op, a, b, c, code),
        RtOp::ListJoin
        | RtOp::ListSort
        | RtOp::ListSortBy
        | RtOp::ListReverse
        | RtOp::ListTake
        | RtOp::ListSkip
        | RtOp::ListIndexOf
        | RtOp::ListMin
        | RtOp::ListMax => crate::rt_text::list_op(op, a, b, code),
        RtOp::BytesNew
        | RtOp::StrBytes
        | RtOp::BytesAt
        | RtOp::BytesLen
        | RtOp::BytesGet
        | RtOp::BytesSlice
        | RtOp::BytesText
        | RtOp::BytesToList
        | RtOp::BytesAsList
        | RtOp::BytesCrc32c
        | RtOp::BytesXxhash64
        | RtOp::BytesConcat => crate::rt_bytes::bytes_op(op, a, b, c, code, line),
        RtOp::FsLines
        | RtOp::FsChunks
        | RtOp::ReaderNext
        | RtOp::ReaderTryNext
        | RtOp::FsScopeEnter
        | RtOp::FsScopeLeave => crate::rt_files::reader_op(op, a, b, code, line),
        RtOp::FsReadText
        | RtOp::FsReadBytes
        | RtOp::FsWriteBytes
        | RtOp::FsWriteDurable
        | RtOp::FsWriteText
        | RtOp::FsAppendText
        | RtOp::FsExists
        | RtOp::FsList
        | RtOp::FsRemove
        | RtOp::FsCreateDir
        | RtOp::TimeNowMs
        | RtOp::TimeMonotonicMs
        | RtOp::TimeSleepMs
        | RtOp::MathSqrt
        | RtOp::MathPow
        | RtOp::MathFloor
        | RtOp::MathCeil
        | RtOp::MathRound
        | RtOp::IoReadLine
        | RtOp::IoReadHidden
        | RtOp::IoIsTerminal
        | RtOp::IoColors
        | RtOp::IoWrite
        | RtOp::CliArgs
        | RtOp::EnvGet
        | RtOp::CliExit
        | RtOp::CliParse => crate::rt_std::std_op(op, a, b, code, line),
        RtOp::TimeWaitUntil => crate::rt_std::wait_until(a, line),
        RtOp::MapGet
        | RtOp::MapGetOr
        | RtOp::MapInsert
        | RtOp::MapRemove
        | RtOp::MapContains
        | RtOp::MapPrefetch
        | RtOp::MapLen
        | RtOp::MapIsEmpty
        | RtOp::MapKeys
        | RtOp::MapValues
        | RtOp::MapEntries
        | RtOp::SharedNew
        | RtOp::SharedGet
        | RtOp::SharedSet
        | RtOp::SharedLock
        | RtOp::SharedUnlock => crate::rt_maps::map_or_shared(op, a, b, c, code, line),
        RtOp::ListContains => {
            let item_ty = match shape(obj(a).ty()) {
                Shape::List(t) => *t,
                _ => Type::I64,
            };
            let found = items_of(a).iter().any(|s| equal(*s as i64, b, item_ty));
            release(a);
            release_slot(b, item_ty);
            flag(found)
        }
        RtOp::FloatBits | RtOp::FloatFromBits => a,
        RtOp::DecRound => {
            let rounded = crate::rt_big_decimal::round(a, b, c != 0, code, line);
            release(a);
            rounded
        }
        RtOp::ErrorAs => {
            let want = match shape(type_of(code)) {
                Shape::Optional(t) => *t,
                _ => Type::I64,
            };
            let found = crate::rt_errors::error_as(a, want);
            release(a);
            found
        }
        RtOp::PtrAlloc | RtOp::PtrFree | RtOp::PtrText | RtOp::PtrReadText => {
            crate::rt_ptr::ptr_op(op, a, b, code, line)
        }
        // Ops of the other groups come through their own entries
        // (`RtOp::entry`).
        _ => panic_with(&format!("`{op:?}` goes through `{}`", op.entry()), line),
    }
}

/// Defines the entry of a feature group but the core (`RtOp::entry`). Each
/// entry is written in its group's own module, so it is compiled apart
/// from the core and only linked into a program that uses the group
/// (decision 0024).
macro_rules! entry {
    ($name:ident, |$op:ident, $a:ident, $b:ident, $c:ident, $code:ident, $line:ident| $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C-unwind" fn $name(
            $op: i64,
            $a: i64,
            $b: i64,
            $c: i64,
            $code: i64,
            $line: i32,
        ) -> i64 {
            let $op = kavach_kir::RtOp::ALL[$op as usize];
            $body
        }
    };
}
pub(crate) use entry;
