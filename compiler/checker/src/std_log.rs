//! The signatures of `std.log`'s built-in functions (spec 14.12): what
//! its metrics, `trace_id` and `traceparent`, written in Kavach, are
//! built on. Like logging, they need no capability.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `log.<name>`, or `None`.
    pub(crate) fn log_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, string) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::String));
        let p = |n: &str, t: Ty| (n.to_string(), t, false);
        Some(match name {
            // The kind (0 a sum, 1 a gauge, 2 a histogram), name and value.
            "metric_id" => (
                vec![
                    p("kind", int),
                    p("name", string),
                    p("value", Ty::Prim(Prim::Float)),
                ],
                Ty::Unit,
            ),
            // Empty outside a span.
            "trace_id_text" | "traceparent_text" => (vec![], string),
            _ => return None,
        })
    }
}
