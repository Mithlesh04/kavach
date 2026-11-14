//! The signatures of the built-in functions `std.fs`'s `FileWriter`
//! (spec 14.3) is built on, by the writer's number. They change files, so
//! they need `Fs.write`.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `fs.writer_*`, or `None`.
    pub(crate) fn writer_signature(&mut self, name: &str) -> Option<Signature> {
        let int = Ty::Prim(Prim::Int);
        let p = |n: &str, t: Ty| (n.to_string(), t, false);
        let e = self.builtin_ty("FsError");
        let fails = |t: Ty| Ty::Result(Box::new(t), Box::new(e.clone()));
        Some(match name {
            "writer_open_id" => (vec![p("path", Ty::Prim(Prim::String))], fails(int)),
            "writer_write_id" => (
                vec![p("id", int), p("data", Ty::Prim(Prim::Bytes))],
                fails(Ty::Unit),
            ),
            "writer_finish_id" => (
                vec![p("id", int.clone()), p("sync", Ty::Prim(Prim::Bool))],
                fails(int),
            ),
            "writer_close_id" => (vec![p("id", int)], Ty::Unit),
            _ => return None,
        })
    }
}
