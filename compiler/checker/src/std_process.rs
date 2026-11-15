//! The signatures of `std.process`'s built-in functions (spec 14.23),
//! and `std.system`'s (14.25):
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
            "peak_bytes_id" | "cpu_ms_id" => (vec![], int),
            _ => return None,
        })
    }

    /// The parameters and result of `system.<name>` (spec 14.25): a fact
    /// by its number, as text or a number (`basic_` ones free), and the
    /// GPUs, four texts each.
    pub(crate) fn system_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, string) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::String));
        let which = vec![("which".to_string(), int.clone(), false)];
        Some(match name {
            "basic_text_id" | "text_id" => (which, string),
            "basic_number_id" | "number_id" => (which, int),
            "gpus_id" | "apps_id" | "ids_id" => (vec![], Ty::List(Box::new(string))),
            // Every part of the machine, by kind: rows of text (14.25).
            "rows_id" => (which, Ty::List(Box::new(string))),
            _ => return None,
        })
    }
}
