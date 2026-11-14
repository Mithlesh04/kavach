//! `std.gpu`'s drawing in the interpreter (spec 14.24), from
//! `kavach_runtime_gpu::draw`, as native code does: screens and frames by
//! number, on windows from `kavach_runtime_window`.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_checker::Prim;
use kavach_runtime_gpu::draw;
use kavach_runtime_window as win;

impl<'a> Interp<'a> {
    /// `gpu.screen_id`, `frame_id` and the rest, with every argument given.
    pub(crate) fn gpu_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => (*n).clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            _ => 0,
        };
        let size = |n: i64| {
            let (w, h) = win::size(n);
            (u32::try_from(w).unwrap_or(0), u32::try_from(h).unwrap_or(0))
        };
        let number = |made: Result<i64, String>, me: &Self| match made {
            Ok(n) => Ok(Value::Int(i128::from(n), Prim::Int)),
            Err(why) => {
                let reason = me.str_value(&why);
                Err(Flow::Fail(me.builtin_variant(
                    "GpuError",
                    "Unavailable",
                    vec![("reason", reason)],
                )))
            }
        };
        let n = int(0);
        match name {
            "screen_id" => {
                return number(draw::screen_on(n, size(n)), self);
            }
            "frame_id" => {
                let (w, h) = size(int(1));
                return number(draw::frame(n, w, h), self);
            }
            "clear_id" => {
                let part = |i: usize| match &args.get(1) {
                    Some(Value::List(rgb)) => match rgb.get(i) {
                        Some(Value::Float(f)) => *f,
                        _ => 0.0,
                    },
                    _ => 0.0,
                };
                draw::clear(n, part(0), part(1), part(2));
            }
            "show_id" => draw::show(n),
            "drop_frame_id" => draw::drop_frame(n),
            _ => draw::close(n),
        }
        Ok(Value::Unit)
    }
}
