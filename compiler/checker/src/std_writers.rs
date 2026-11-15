//! The signatures of the built-in functions `std.fs`'s `FileWriter` and
//! `Watcher` (spec 14.3) are built on, by their numbers. A writer changes
//! files, so it needs `Fs.write`; a watcher reads, so `Fs.read`.

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
            "writer_close_id" | "watch_close_id" => (vec![p("id", int)], Ty::Unit),
            // Watching (spec 14.3): each change as its kind's digit, a space
            // and the path.
            "watch_open_id" => (vec![p("path", Ty::Prim(Prim::String))], fails(int)),
            "watch_next_id" => (vec![p("id", int)], Ty::Prim(Prim::String)),
            "watch_poll_id" => (
                vec![p("id", int)],
                Ty::List(Box::new(Ty::Prim(Prim::String))),
            ),
            _ => return None,
        })
    }

    /// The parameters and result of `app.<name>` (spec 14.30), or `None`.
    pub(crate) fn app_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, string) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::String));
        let p = |n: &str, t: Ty| (n.to_string(), t, false);
        let e = self.builtin_ty("AppError");
        let fails = |t: Ty| Ty::Result(Box::new(t), Box::new(e.clone()));
        Some(match name {
            // The kind (0 data, 1 settings, 2 cache) and the program's name.
            "dir_id" => (
                vec![p("kind", int), p("name", string.clone())],
                fails(string),
            ),
            "crash_id" => (vec![p("folder", string)], fails(Ty::Unit)),
            // Which dialog, its title and its text: the path chosen, or "".
            "dialog_id" => (
                vec![
                    p("which", int),
                    p("title", string.clone()),
                    p("text", string.clone()),
                ],
                string,
            ),
            _ => return None,
        })
    }
}
