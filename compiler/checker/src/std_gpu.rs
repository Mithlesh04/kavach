//! The signatures of `std.gpu`'s built-in functions (spec 14.24): what
//! `gpu.screen` and the methods of `Screen` and `Frame`, written in
//! Kavach, are built on, by number.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `gpu.<name>`, or `None` when there is
    /// no such function.
    pub(crate) fn draw_signature(&mut self, name: &str) -> Option<Signature> {
        let int = Ty::Prim(Prim::Int);
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let id = p("id", &int);
        let failing = |c: &mut Self| {
            let error = c.builtin_ty("GpuError");
            Ty::Result(Box::new(int.clone()), Box::new(error))
        };
        Some(match name {
            // The window's number.
            "screen_id" => (vec![p("window", &int)], failing(self)),
            // The screen's number and its window's.
            "frame_id" => (vec![id, p("window", &int)], failing(self)),
            // Red, green and blue.
            "clear_id" => (
                vec![id, p("rgb", &Ty::List(Box::new(Ty::Prim(Prim::Float))))],
                Ty::Unit,
            ),
            "show_id" | "drop_frame_id" | "close_id" => (vec![id], Ty::Unit),
            _ => return None,
        })
    }
}
