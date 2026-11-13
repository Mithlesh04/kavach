//! `std.text`'s Unicode normal forms in native code (spec 14.9, decision
//! 0102), from `kavach_runtime_core::normalize`, as the interpreter does.
//! An entry of its own, so a program that doesn't use them links none of
//! Unicode's tables (decision 0024).

use crate::heap::release;
use crate::rt::type_of;
use crate::shapes::{new_str, str_of};
use kavach_runtime_core::normalize::normalize;

// `a` is the text (owned), `b` the form: 0 NFC, 1 NFD, 2 NFKC, 3 NFKD.
crate::rt_data::entry!(kv_rt_normalize, |_op, a, b, _c, code, _line| {
    let text = str_of(a);
    release(a);
    new_str(normalize(&text, b), type_of(code))
});
