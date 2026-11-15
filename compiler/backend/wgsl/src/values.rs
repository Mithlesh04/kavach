//! Values' parts in a kernel: a field of a record, tuple or `std.vec`
//! value, and each type's zero value.

use kavach_kir::{Program, Type};

/// Field `k` of `b`, of record or tuple type `t`: by index for a vector
/// or matrix of `std.vec` (`vec.rs`).
pub(crate) fn field(program: &Program, t: Type, b: &str, k: i128) -> String {
    match crate::vec::vec_type(program, t) {
        Some(_) => format!("({b})[{k}]"),
        None => format!("({b}).f{k}"),
    }
}

pub(crate) fn zero(program: &Program, t: Type) -> String {
    match t {
        Type::I32 => "i32(0)".into(),
        Type::U32 => "0u".into(),
        Type::F32 => "f32(0.0)".into(),
        // A struct's or a `std.vec` type's zero value: every field zero.
        Type::Ref(id) => match crate::vec::vec_type(program, t) {
            Some(v) => format!("{v}()"),
            None => format!("S{id}()"),
        },
        _ => "false".into(),
    }
}
