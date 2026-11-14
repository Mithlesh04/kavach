//! The signatures of `std.window`'s built-in functions (spec 14.22):
//! what `window.open` and the methods of `AppWindow`, written in Kavach,
//! are built on, by the window's number.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `window.<name>`, or `None` when there
    /// is no such function.
    pub(crate) fn window_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, string) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::String));
        let (boolean, float) = (Ty::Prim(Prim::Bool), Ty::Prim(Prim::Float));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let id = p("id", &int);
        Some(match name {
            "open_id" => {
                let error = self.builtin_ty("WindowError");
                (
                    vec![p("title", &string), p("width", &int), p("height", &int)],
                    Ty::Result(Box::new(int), Box::new(error)),
                )
            }
            "events_id" => (vec![id, p("wait", &boolean)], Ty::List(Box::new(float))),
            "texts_id" => (vec![id], Ty::List(Box::new(string))),
            "size_id" => (vec![id], Ty::List(Box::new(int))),
            "scale_id" => (vec![id], float),
            "set_title_id" => (vec![id, p("title", &string)], Ty::Unit),
            "set_fullscreen_id" | "lock_cursor_id" => (vec![id, p("on", &boolean)], Ty::Unit),
            "listen_id" | "close_id" => (vec![id], Ty::Unit),
            _ => return None,
        })
    }
}
