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
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

/// A pipeline: its vertex and fragment fns, and whether it has depth.
pub type PipelineKey = (DefId, DefId, bool);

/// Each pipeline's shaders, by its key: the WGSL the compiler writes,
/// which the interpreter can't (spec 15.3).
fn pipelines() -> &'static Mutex<HashMap<PipelineKey, String>> {
    static PIPELINES: OnceLock<Mutex<HashMap<PipelineKey, String>>> = OnceLock::new();
    PIPELINES.get_or_init(Mutex::default)
}

/// Gives the interpreter the program's pipelines' shaders, before it runs
/// (`kavach run`, from `compiler/backend/wgsl`).
pub fn give_pipelines(shaders: HashMap<PipelineKey, String>) {
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

/// The bytes of a `[UInt8]`.
fn bytes(v: Option<&Value<'_>>) -> Vec<u8> {
    match v {
        Some(Value::List(items)) => (items.iter())
            .map(|x| match x {
                Value::Int(n, _) => *n as u8,
                _ => 0,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Each texture's number, its first field.
fn texture_ids(v: Option<&Value<'_>>) -> Vec<i64> {
    let Some(Value::List(items)) = v else {
        return Vec::new();
    };
    (items.iter())
        .filter_map(|t| match t {
            Value::Record { fields, .. } => match fields.first().map(|f| &f.value) {
                Some(Value::Int(n, _)) => Some(*n as i64),
                _ => None,
            },
            _ => None,
        })
        .collect()
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
                let key = match args {
                    [Value::Func(v), Value::Func(f)] => Some((*v, *f, false)),
                    [Value::Func(v), Value::Func(f), Value::Bool(d)] => Some((*v, *f, *d)),
                    _ => None,
                };
                let text = key.and_then(|k| {
                    let all = pipelines().lock().unwrap_or_else(|e| e.into_inner());
                    all.get(&k).cloned()
                });
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
                let textures = texture_ids(args.get(5));
                let indices = match args.get(6) {
                    Some(Value::List(_)) => {
                        let mut ix = Vec::new();
                        words(&args[6], &mut ix);
                        Some(ix)
                    }
                    _ => None,
                };
                let made = kavach_runtime_gpu::render::Draw {
                    pipeline: int(1),
                    parts: (
                        kavach_runtime_gpu::render::Items::Words(&parts[0]),
                        kavach_runtime_gpu::render::Items::Words(&parts[1]),
                        &parts[2],
                    ),
                    textures: &textures,
                    indices: indices.as_deref(),
                };
                if let Err(why) = draw::draw(n, made) {
                    return Err(self.panic(why, span));
                }
            }
            "texture_id" => {
                let fit = |v: i64| u32::try_from(v).unwrap_or(0);
                let pixels = bytes(args.get(2));
                let made = kavach_runtime_gpu::texture::texture(fit(n), fit(int(1)), &pixels);
                return number(made, self);
            }
            "drop_texture_id" => kavach_runtime_gpu::texture::drop_texture(n),
            "compressed_id" => {
                let file = match args.first() {
                    Some(Value::Bytes(b)) => Rc::clone(b),
                    _ => Rc::from(&[][..]),
                };
                return match kavach_runtime_gpu::texture::compressed(&file) {
                    Ok((t, w, h)) => {
                        let parts = [i128::from(t), i128::from(w), i128::from(h)];
                        let items: Vec<Value<'a>> =
                            parts.iter().map(|v| Value::Int(*v, Prim::Int)).collect();
                        Ok(Value::List(Rc::new(items.into())))
                    }
                    Err((variant, why)) => {
                        let reason = self.str_value(&why);
                        Err(Flow::Fail(self.builtin_variant(
                            "GpuError",
                            variant,
                            vec![("reason", reason)],
                        )))
                    }
                };
            }
            "show_id" => draw::show(n),
            "drop_frame_id" => draw::drop_frame(n),
            _ => draw::close(n),
        }
        Ok(Value::Unit)
    }
}
