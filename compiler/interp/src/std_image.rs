//! `std.image` in the interpreter (spec 14.29), from
//! `kavach_runtime_image`, as native code does.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_image as image;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// `image.decode_id`, `size_id`, `pixels_id` and `png_id`.
    pub(crate) fn image_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => *n as i64,
            _ => 0,
        };
        let bytes = |i: usize| match args.get(i) {
            Some(Value::Bytes(b)) => Rc::clone(b),
            _ => Rc::from(&[][..]),
        };
        Ok(match name {
            "decode_id" => match image::decode_kept(&bytes(0)) {
                Ok(n) => Value::Int(i128::from(n), Prim::Int),
                Err(why) => {
                    let reason = self.str_value(&why);
                    return Err(Flow::Fail(self.builtin_variant(
                        "ImageError",
                        "BadImage",
                        vec![("reason", reason)],
                    )));
                }
            },
            "size_id" => {
                let (w, h) = image::size(int(0));
                let both = [w, h].map(|v| Value::Int(i128::from(v), Prim::Int));
                Value::List(Rc::new(both.to_vec().into()))
            }
            "pixels_id" => Value::Bytes(image::take_pixels(int(0)).into()),
            _ => {
                let fit = |v: i64| u32::try_from(v).unwrap_or(0);
                match image::png(fit(int(0)), fit(int(1)), &bytes(2)) {
                    Ok(file) => Value::Bytes(file.into()),
                    Err(why) => return Err(self.panic(why, span)),
                }
            }
        })
    }
}
