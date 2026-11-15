//! The signatures of `std.window`'s built-in functions (spec 14.22):
//! what `window.open` and the methods of `AppWindow`, written in Kavach,
//! are built on, by the window's number; `std.input`'s (spec 14.26),
//! under `input.pads()` and `pad.rumble`; and `std.audio`'s (14.27).

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

    /// The parameters and result of `input.<name>`, or `None`.
    pub(crate) fn input_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, float) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::Float));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let list = |t: &Ty| Ty::List(Box::new(t.clone()));
        Some(match name {
            "start_id" => (vec![], Ty::Unit),
            // Eight numbers a pad, and the pads' names.
            "pads_id" => (vec![], list(&float)),
            "names_id" => (vec![], list(&Ty::Prim(Prim::String))),
            // A pad, its two motors' strengths, and milliseconds.
            "rumble_id" => (
                vec![
                    p("pad", &int),
                    p("motors", &list(&float)),
                    p("millis", &int),
                ],
                Ty::Unit,
            ),
            _ => return None,
        })
    }

    /// The parameters and result of `audio.<name>` (spec 14.27), or
    /// `None`: sounds and voices by number.
    pub(crate) fn audio_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, float) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::Float));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let list = |t: &Ty| Ty::List(Box::new(t.clone()));
        let failing = |c: &mut Self| {
            let error = c.builtin_ty("AudioError");
            Ty::Result(Box::new(Ty::Prim(Prim::Int)), Box::new(error))
        };
        let id = p("id", &int);
        Some(match name {
            "sound_id" => (
                vec![
                    p("samples", &list(&Ty::Prim(Prim::Float32))),
                    p("rate", &int),
                    p("channels", &int),
                ],
                int.clone(),
            ),
            "decode_id" => (vec![p("bytes", &Ty::Prim(Prim::Bytes))], failing(self)),
            "info_id" => (vec![id], list(&int)),
            "drop_sound_id" | "stop_id" => (vec![id], Ty::Unit),
            // The sound, its volume and pan, and whether it loops.
            "play_id" => (
                vec![
                    id,
                    p("settings", &list(&float)),
                    p("looped", &Ty::Prim(Prim::Bool)),
                ],
                failing(self),
            ),
            // A voice, one value, and which: 0 the volume, 1 the pan.
            "set_id" => (
                vec![id, p("value", &list(&float)), p("which", &int)],
                Ty::Unit,
            ),
            "playing_id" => (vec![id], Ty::Prim(Prim::Bool)),
            _ => return None,
        })
    }

    /// The parameters and result of `image.<name>` (spec 14.29), or `None`.
    pub(crate) fn image_signature(&mut self, name: &str) -> Option<Signature> {
        let (int, bytes) = (Ty::Prim(Prim::Int), Ty::Prim(Prim::Bytes));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        Some(match name {
            "decode_id" => {
                let error = self.builtin_ty("ImageError");
                (
                    vec![p("bytes", &bytes)],
                    Ty::Result(Box::new(int), Box::new(error)),
                )
            }
            "size_id" => (vec![p("id", &int)], Ty::List(Box::new(int))),
            "pixels_id" => (vec![p("id", &int)], bytes),
            "png_id" => (
                vec![p("width", &int), p("height", &int), p("pixels", &bytes)],
                bytes,
            ),
            _ => return None,
        })
    }
}
