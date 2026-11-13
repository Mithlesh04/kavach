//! The signatures of `std.compress`'s built-in functions (spec 14.21):
//! what `encode` and `decode`, written in Kavach, are built on.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `compress.<name>`, or `None` when
    /// there is no such function.
    pub(crate) fn compress_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, bytes) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::Bytes));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        Some(match name {
            "encode_bytes" => (
                vec![p("data", &bytes), p("format", &int), p("level", &int)],
                bytes,
            ),
            "decode_bytes" => {
                let error = self.builtin_ty("CompressError");
                (
                    vec![p("data", &bytes), p("format", &int), p("most", &int)],
                    Ty::Result(Box::new(bytes), Box::new(error)),
                )
            }
            _ => return None,
        })
    }
}
