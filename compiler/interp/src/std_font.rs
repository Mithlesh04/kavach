//! `std.font` in the interpreter (spec 14.31), from
//! `kavach_runtime_font`, as native code does.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_font as font;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `font.metrics_id`, `shape_id`, `atlas_id`, `atlas_boxes_id` and
    /// `atlas_pixels_id`.
    pub(crate) fn font_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let bytes = match args.first() {
            Some(Value::Bytes(b)) => Rc::clone(b),
            _ => Rc::from(&[][..]),
        };
        let id = match args.first() {
            Some(Value::Int(n, _)) => *n as i64,
            _ => 0,
        };
        let text = match args.get(1) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let size = match args.get(2) {
            Some(Value::Float(f)) => *f,
            _ => 0.0,
        };
        let floats = |v: Vec<f64>| {
            let items: Vec<Value<'a>> = v.into_iter().map(Value::Float).collect();
            Value::List(Rc::new(items.into()))
        };
        Ok(match name {
            "metrics_id" => match font::metrics(&bytes) {
                Ok(m) => floats(m.to_vec()),
                Err(why) => {
                    let reason = self.str_value(&why);
                    return Err(Flow::Fail(self.builtin_variant(
                        "FontError",
                        "BadFont",
                        vec![("reason", reason)],
                    )));
                }
            },
            "shape_id" => floats(font::shape_flat(&bytes, &text, size)),
            "atlas_id" => match font::atlas_kept(&bytes, &text, size) {
                Ok(n) => Value::Int(i128::from(n), Prim::Int),
                Err(why) => return Err(self.panic(why, span)),
            },
            "atlas_boxes_id" => {
                let items: Vec<Value<'a>> = (font::atlas_boxes(id).into_iter())
                    .map(|n| Value::Int(i128::from(n), Prim::Int))
                    .collect();
                Value::List(Rc::new(items.into()))
            }
            _ => Value::Bytes(font::take_atlas_pixels(id).into()),
        })
    }
}
