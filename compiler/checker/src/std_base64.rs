//! The signatures of `std.base64` (spec 14.20): pure, so no capability.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `base64.<name>`, or `None` when there
    /// is no such function.
    pub(crate) fn base64_signature(&mut self, name: &str) -> Option<Signature> {
        let (string, bytes) = (Ty::Prim(Prim::String), Ty::Prim(Prim::Bytes));
        let boolean = Ty::Prim(Prim::Bool);
        let p = |n: &str, t: &Ty, default: bool| (n.to_string(), t.clone(), default);
        Some(match name {
            "encode" => (
                vec![
                    p("data", &bytes, false),
                    p("url", &boolean, true),
                    p("pad", &boolean, true),
                ],
                string,
            ),
            "decode" => {
                let error = self.builtin_ty("Base64Error");
                (
                    vec![p("text", &string, false), p("url", &boolean, true)],
                    Ty::Result(Box::new(bytes), Box::new(error)),
                )
            }
            _ => return None,
        })
    }
}
