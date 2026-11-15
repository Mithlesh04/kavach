//! `std.gpu`'s drawing in the interpreter (spec 14.24), from
//! `kavach_runtime_gpu::draw`, as native code does: screens and frames by
//! number, on windows from `kavach_runtime_window`.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_resolver::DefId;
use kavach_runtime_gpu::draw;
use kavach_runtime_window as win;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Each pipeline's shaders, by its vertex and fragment fns: the WGSL the
/// compiler writes, which the interpreter can't (spec 15.3).
fn pipelines() -> &'static Mutex<HashMap<(DefId, DefId), String>> {
    static PIPELINES: OnceLock<Mutex<HashMap<(DefId, DefId), String>>> = OnceLock::new();
    PIPELINES.get_or_init(Mutex::default)
}

/// Gives the interpreter the program's pipelines' shaders, before it runs
/// (`kavach run`, from `compiler/backend/wgsl`).
pub fn give_pipelines(shaders: HashMap<(DefId, DefId), String>) {
    pipelines()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .extend(shaders);
}

/// A vertex's, instance's or shared value's fields, as the GPU reads them.
fn words(v: &Value<'_>, out: &mut Vec<u32>) {
    match v {
        Value::Record { fields, .. } => fields.iter().for_each(|f| words(&f.value, out)),
        Value::List(items) => items.iter().for_each(|x| words(x, out)),
        Value::Float32(f) => out.push(f.to_bits()),
        Value::Float(f) => out.push((*f as f32).to_bits()),
        Value::Int(n, _) => out.push(*n as i64 as u32),
        Value::Bool(b) => out.push(u32::from(*b)),
        _ => {}
    }
}

impl<'a> Interp<'a> {
    /// `gpu.screen_id`, `frame_id` and the rest, with every argument given.
    pub(crate) fn gpu_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
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
            "pipeline" => {
                let text = match args {
                    [Value::Func(v), Value::Func(f)] => pipelines()
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&(*v, *f))
                        .cloned(),
                    _ => None,
                };
                let made = text.ok_or_else(|| {
                    "this pipeline's shaders weren't written: run the program with `kavach run`"
                        .to_string()
                });
                return match made.and_then(|t| kavach_runtime_gpu::render::pipeline(&t)) {
                    Ok(n) => Ok(Value::Int(i128::from(n), Prim::Int)),
                    Err(why) => Err(self.panic(why, span)),
                };
            }
            "draw_id" => {
                let mut parts = [Vec::new(), Vec::new(), Vec::new()];
                for (k, part) in parts.iter_mut().enumerate() {
                    if let Some(v) = args.get(2 + k) {
                        words(v, part);
                    }
                }
                if let Err(why) = draw::draw(
                    n,
                    int(1),
                    (
                        kavach_runtime_gpu::render::Items::Words(&parts[0]),
                        kavach_runtime_gpu::render::Items::Words(&parts[1]),
                        &parts[2],
                    ),
                ) {
                    return Err(self.panic(why, span));
                }
            }
            "show_id" => draw::show(n),
            "drop_frame_id" => draw::drop_frame(n),
            _ => draw::close(n),
        }
        Ok(Value::Unit)
    }
}
