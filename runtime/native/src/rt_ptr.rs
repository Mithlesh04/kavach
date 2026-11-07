//! Raw memory for `unsafe` code (spec 6.4) and text passed to C (10.7).
//! The memory is C's own (`calloc` and `free`), so C code may free what
//! Kavach allocated and the other way round. Reads and writes through a
//! `Ptr` don't come here: the backends load and store in place
//! (`compiler/kir/src/lower_ptr.rs`).

use crate::heap::release;
use crate::rt::panic_with;
use crate::shapes::with_str;
use kavach_kir::RtOp;

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut std::ffi::c_void;
    fn free(p: *mut std::ffi::c_void);
}

/// `alloc(a)` of items of `b` bytes; `free(a)`; the text `a` (owned)
/// copied for C, with a zero byte after it; C's text at `a`, as an
/// optional text of type `code`.
pub fn ptr_op(op: RtOp, a: i64, b: i64, code: i64, line: i32) -> i64 {
    match op {
        RtOp::PtrReadText => {
            if a == 0 {
                return 0;
            }
            // SAFETY: none possible: the program promised, inside
            // `unsafe`, that `a` points to text ending in a zero byte
            // (spec 6.4).
            let bytes = unsafe { std::ffi::CStr::from_ptr(a as *const std::ffi::c_char) };
            let Ok(text) = bytes.to_str() else {
                panic_with("text from C that isn't UTF-8 (spec 6.4)", line);
            };
            let ty = match crate::shapes::shape(crate::rt::type_of(code)) {
                kavach_kir::Shape::Optional(t) => *t,
                _ => crate::rt::type_of(code),
            };
            crate::shapes::new_text(text, ty)
        }
        RtOp::PtrAlloc => {
            let (Ok(count), Ok(size)) = (usize::try_from(a), usize::try_from(b)) else {
                panic_with(&format!("`alloc` of {a} items"), line);
            };
            // SAFETY: `calloc` takes any sizes, and says when it can't.
            let p = unsafe { calloc(count.max(1), size) };
            if p.is_null() {
                panic_with(
                    &format!("out of memory: `alloc` of {count} items of {size} bytes"),
                    line,
                );
            }
            p as i64
        }
        RtOp::PtrFree => {
            // SAFETY: none, and none is possible: `free` is `unsafe` in
            // Kavach too, and the program promised `a` came from `calloc`
            // or C (spec 6.4).
            unsafe { free(a as *mut std::ffi::c_void) };
            0
        }
        _ => c_copy(a, line),
    }
}

/// The text `a` (owned) copied for C, with a zero byte after it.
fn c_copy(a: i64, line: i32) -> i64 {
    let text = with_str(a, |s| s.as_bytes().to_vec());
    release(a);
    if text.contains(&0) {
        panic_with("text passed to C has a zero byte in it (spec 10.7)", line);
    }
    // SAFETY: `calloc` gives `len + 1` zeroed bytes or null; the copy
    // fills all but the last, which stays the zero C needs.
    unsafe {
        let p = calloc(text.len() + 1, 1).cast::<u8>();
        if p.is_null() {
            panic_with("out of memory: text passed to C", line);
        }
        std::ptr::copy_nonoverlapping(text.as_ptr(), p, text.len());
        p as i64
    }
}

/// A `String` argument of a library's exported function (spec 10.10):
/// the caller's C text, copied into a new text of type `code`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_text_in(p: i64, code: i64) -> i64 {
    if p == 0 {
        panic_with(
            "a library function got a null pointer for text (spec 10.10)",
            0,
        );
    }
    // SAFETY: none possible: the caller promised, as C does, that `p`
    // points to text ending in a zero byte.
    let bytes = unsafe { std::ffi::CStr::from_ptr(p as *const std::ffi::c_char) };
    let Ok(text) = bytes.to_str() else {
        panic_with(
            "text given to a library function isn't UTF-8 (spec 10.10)",
            0,
        );
    };
    crate::shapes::new_text(text, crate::rt::type_of(code))
}

/// A `String` result of a library's exported function (owned): a C copy,
/// which the caller gives back to `kv_lib_free`.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_text_out(p: i64) -> i64 {
    c_copy(p, 0)
}

thread_local! {
    /// The text of the last error a library function failed with on this
    /// thread (decision 0035).
    static LAST_ERROR: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// A library function failed with `error` (owned): its number for C,
/// the variant counting from 1 (1 for an error that isn't a union), and
/// its text kept for `<package>_last_error` (spec 10.10).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_error(error: i64) -> i64 {
    let ty = crate::heap::obj(error).ty();
    let code = match crate::shapes::shape(ty) {
        kavach_kir::Shape::Union { .. } => i64::from(crate::heap::tag_of(error)) + 1,
        _ => 1,
    };
    let text = crate::display::show_error(error);
    release(error);
    LAST_ERROR.set(Some(text));
    code
}

/// `<package>_last_error`: the calling thread's last error as C text,
/// for `<package>_free`, or null before any.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_last_error() -> i64 {
    let last = LAST_ERROR.with_borrow(|e| e.clone());
    let Some(text) = last else {
        return 0;
    };
    // SAFETY: `calloc` gives `len + 1` zeroed bytes or null; the copy
    // fills all but the last, which stays the zero C needs.
    unsafe {
        let p = calloc(text.len() + 1, 1).cast::<u8>();
        if !p.is_null() {
            std::ptr::copy_nonoverlapping(text.as_ptr(), p, text.len());
        }
        p as i64
    }
}

/// `<package>_free`: frees text a library gave (spec 10.10).
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_lib_free(p: i64) {
    if p != 0 {
        // SAFETY: none possible: the caller promised `p` came from this
        // library's `kv_lib_text_out`, which got it from `calloc`.
        unsafe { free(p as *mut std::ffi::c_void) };
    }
}
