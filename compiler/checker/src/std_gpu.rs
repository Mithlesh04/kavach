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
            "show_id" | "drop_frame_id" | "close_id" | "drop_texture_id" => (vec![id], Ty::Unit),
            // A texture's size and its pixels (spec 15.3).
            "texture_id" => {
                let bytes = Ty::List(Box::new(Ty::Prim(Prim::UInt8)));
                (
                    vec![p("width", &int), p("height", &int), p("pixels", &bytes)],
                    failing(self),
                )
            }
            // A frame's number, a pipeline, its vertices, instances and
            // shared value, the textures (spec 15.3) and the indices.
            "draw_id" => {
                let (v, i, s, t) = (self.fresh(), self.fresh(), self.fresh(), self.fresh());
                let pipeline = Ty::Builtin("Pipeline", vec![v.clone(), i.clone(), s.clone(), t]);
                let texture = self.builtin_ty("Texture");
                let list = |t: &Ty| Ty::List(Box::new(t.clone()));
                let shared = Ty::Optional(Box::new(s));
                (
                    vec![
                        id,
                        p("pipeline", &pipeline),
                        p("vertices", &list(&v)),
                        p("instances", &list(&i)),
                        p("shared", &shared),
                        p("textures", &list(&texture)),
                        p(
                            "indices",
                            &Ty::Optional(Box::new(list(&Ty::Prim(Prim::UInt32)))),
                        ),
                    ],
                    Ty::Unit,
                )
            }
            _ => return None,
        })
    }
}
