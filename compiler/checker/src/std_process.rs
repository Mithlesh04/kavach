//! The signatures of `std.process`'s built-in functions (spec 14.23):
//! what `process.run`, `process.start` and the methods of `Child`,
//! written in Kavach, are built on, by the child's number.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `process.<name>`, or `None` when
    /// there is no such function.
    pub(crate) fn process_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, string) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::String));
        let texts = Ty::List(Box::new(string.clone()));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let id = p("id", &int);
        Some(match name {
            // `setup` is the folder, then the environment's names and
            // values in turn.
            "start_id" => {
                let error = self.builtin_ty("ProcessError");
                (
                    vec![p("program", &string), p("args", &texts), p("setup", &texts)],
                    Ty::Result(Box::new(int), Box::new(error)),
                )
            }
            // `["out", line]`, `["err", line]`, or `[]` at the end.
            "next_id" => (vec![id], texts),
            // The code, processor milliseconds and peak bytes.
            "exit_id" => (vec![id], Ty::List(Box::new(int))),
            "kill_id" | "close_id" => (vec![id], Ty::Unit),
            "peak_bytes_id" => (vec![], int),
            _ => return None,
        })
    }
}
