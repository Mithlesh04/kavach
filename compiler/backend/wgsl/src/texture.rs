//! Textures in a fragment fn (spec 15.3): a `Texture` parameter is a
//! `texture_2d<f32>`, and `t.sample(u, v)` is `ksample`, which reads it
//! through the pipeline's one sampler (`runtime/gpu/src/texture.rs`).

use crate::expr::{Body, Writer};
use kavach_kir::{Expr, Local, Type};

impl<'p> Body<'p> {
    /// Whether `t` is `Texture`.
    pub(crate) fn texture_ty(&self, t: Type) -> bool {
        matches!((t, self.texture), (Type::Ref(id), Some(tex)) if id == tex)
    }

    /// Whether local `l` holds a texture.
    pub(crate) fn is_texture(&self, l: Local) -> bool {
        (self.f.locals.get(l as usize)).is_some_and(|t| self.texture_ty(*t))
    }

    /// `t.sample(u, v)`, giving a color of type `color`.
    pub(crate) fn sample(
        &mut self,
        w: &mut Writer<'p>,
        args: &[Expr],
        color: Type,
    ) -> Option<String> {
        let [t, u, v] = args else { return None };
        let Type::Ref(id) = color else { return None };
        w.name_of(color)?;
        w.sampled = Some(id);
        let (t, u, v) = (self.expr(w, t)?, self.expr(w, u)?, self.expr(w, v)?);
        Some(format!("ksample({t}, {u}, {v})"))
    }
}

/// The sampler and the `count` textures a fragment fn reads, and
/// `ksample`, giving colors of shape `sampled`.
pub(crate) fn declarations(count: usize, sampled: Option<u32>) -> String {
    let mut text = String::from("@group(1) @binding(0) var ksampler: sampler;\n");
    for k in 0..count {
        text.push_str(&format!(
            "@group(1) @binding({}) var ktex{k}: texture_2d<f32>;\n",
            k + 1
        ));
    }
    if let Some(s) = sampled {
        text.push_str(&format!(
            "fn ksample(t: texture_2d<f32>, u: f32, v: f32) -> S{s} {{\n    let c = textureSampleLevel(t, ksampler, vec2<f32>(u, v), 0.0);\n    return S{s}(c.x, c.y, c.z, c.w);\n}}\n"
        ));
    }
    text
}
