//! `gpu vertex fn` and `gpu fragment fn` (spec 15.3): what each takes and
//! gives; `gpu.pipeline(v, f)`, typed by what `v` takes and whether `f`
//! takes textures; and the arguments of `frame.draw`, which must give what
//! the two take.

use crate::checker::{Checker, Shape};
use crate::gpu::vector;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

fn is_f32(t: &Ty) -> bool {
    matches!(t, Ty::Prim(Prim::Float32))
}

/// What a vertex's or instance's field may be: what a GPU reads from a
/// buffer as it is.
fn attribute(t: &Ty) -> bool {
    matches!(t, Ty::Prim(Prim::Float32 | Prim::Int32 | Prim::UInt32))
}

/// Four `Float32`s: a corner's `position`, or a pixel's color.
fn four(t: &Ty) -> bool {
    vector(t) && matches!(t, Ty::Tuple(ts) if ts.len() == 4)
}

const TAKES: &str = "a `gpu vertex fn` takes the vertex, then the instance and the value shared by the draw if it has them: one to three records of 1 to 8 `Float32`, `Int32` or `UInt32` fields (spec 15.3)";
const CORNER: &str = "a record with the field `position: (Float32, Float32, Float32, Float32)` and at most 8 more fields, each a `Float32` (spec 15.3)";

impl Checker<'_> {
    /// The fields of `t`, if it is a record with no type arguments.
    fn record_fields(&self, t: &Ty) -> Option<Vec<(String, Ty)>> {
        let Ty::Adt(def, args) = t else {
            return None;
        };
        match self.types.get(def).map(|i| &i.shape) {
            Some(Shape::Record(fs)) if args.is_empty() => Some(
                (fs.iter())
                    .map(|f| (f.name.clone(), self.defaulted(&f.ty)))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// A vertex, an instance or a shared value.
    fn draw_input(&self, t: &Ty) -> bool {
        self.record_fields(t)
            .is_some_and(|fs| (1..=8).contains(&fs.len()) && fs.iter().all(|(_, t)| attribute(t)))
    }

    /// What a vertex fn gives and a fragment fn takes.
    fn corner(&self, t: &Ty) -> bool {
        self.record_fields(t).is_some_and(|fs| {
            fs.len() <= 9
                && fs
                    .iter()
                    .filter(|(n, t)| n == "position" && self.four_floats(t))
                    .count()
                    == 1
                && fs
                    .iter()
                    .all(|(n, t)| (n == "position" && self.four_floats(t)) || is_f32(t))
        })
    }

    /// A vertex or fragment fn's parameters and result.
    pub(crate) fn stage_signature(&mut self, f: &FnDecl, def: DefId) {
        let Some(sig) = self.fns.get(&def).cloned() else {
            return;
        };
        let params: Vec<Ty> = sig.params.iter().map(|p| self.inf.resolve(&p.ty)).collect();
        let ret = self.inf.resolve(&sig.ret);
        let name = &f.name.name;
        if f.gpu_stage == GpuStage::Vertex {
            if !(1..=3).contains(&params.len()) || !params.iter().all(|t| self.draw_input(t)) {
                self.error(f.name.span, TAKES);
            }
            if !self.corner(&ret) {
                self.error(f.name.span, format!("`{name}` must give {CORNER}"));
            }
        } else {
            if params.is_empty() || !self.corner(&params[0]) {
                self.error(
                    f.name.span,
                    format!("a `gpu fragment fn` takes what the vertex fn gives: {CORNER}"),
                );
            }
            let texture = self.builtin_ty("Texture");
            if params.iter().skip(1).any(|t| *t != texture) {
                self.error(f.name.span, "after what the vertex fn gives, a `gpu fragment fn` takes only `Texture`s (spec 15.3)");
            }
            if !self.four_floats(&ret) {
                self.error(f.name.span, format!("`{name}` must give the pixel's color: `(Float32, Float32, Float32, Float32)`, red, green, blue and opacity, each from 0 to 1 (spec 15.3)"));
            }
        }
    }

    /// `gpu.pipeline(v, f)`: a `Pipeline<V, I, S, T>`, where `V`, `I` and
    /// `S` are what `v` takes, `NoInput` for what it doesn't, and `T` is
    /// `Texture` when `f` takes textures. `depth:` is `true` or `false`.
    pub(crate) fn pipeline_call(&mut self, all: &[Arg], span: Span) -> Ty {
        for a in all {
            self.value(&a.value, None);
        }
        let args: Vec<Arg> = all.iter().filter(|a| a.name.is_none()).cloned().collect();
        for a in all.iter().filter(|a| a.name.is_some()) {
            let literal = matches!(a.value.kind, ExprKind::Bool(_));
            if a.name.as_ref().is_some_and(|n| n.name == "depth") && !literal {
                self.error(a.value.span, "`depth:` is written `true` or `false`: it is part of the pipeline, compiled before it draws (spec 15.3)");
            } else if !literal {
                self.error(a.value.span, "`gpu.pipeline` takes a `gpu vertex fn`, a `gpu fragment fn` and `depth:` (spec 15.3)");
            }
        }
        let stage = |c: &Self, a: &Arg| match &a.value.kind {
            ExprKind::Name(id) => c
                .res
                .uses
                .get(&id.span.start)
                .and_then(|d| Some((*d, *c.gpu_stages.get(d)?))),
            _ => None,
        };
        let found = match &args[..] {
            [v, f] => (stage(self, v), stage(self, f)),
            _ => (None, None),
        };
        let (Some((vd, GpuStage::Vertex)), Some((fd, GpuStage::Fragment))) = found else {
            self.error(span, "`gpu.pipeline` takes a `gpu vertex fn` and a `gpu fragment fn`, by name: `gpu.pipeline(place, glow)` (spec 15.3)");
            return Ty::Error;
        };
        let (Some(vs), Some(fs)) = (self.fns.get(&vd).cloned(), self.fns.get(&fd).cloned()) else {
            return Ty::Error;
        };
        let gives = self.inf.resolve(&vs.ret);
        let takes = fs.params.first().map(|p| self.inf.resolve(&p.ty));
        if takes.as_ref() != Some(&gives) {
            let (v, f) = (&self.res.defs[vd].name, &self.res.defs[fd].name);
            let (gives, takes) = (
                self.show(&gives),
                takes.map_or(String::new(), |t| self.show(&t)),
            );
            self.error(args[1].value.span, format!("`{f}` takes a `{takes}`, and `{v}` gives a `{gives}`: a fragment fn takes what the vertex fn gives (spec 15.3)"));
        }
        self.pipelines.insert((span.start, span.end), (vd, fd));
        let nothing = self.builtin_ty("NoInput");
        let mut inputs: Vec<Ty> = (0..3)
            .map(|k| {
                vs.params
                    .get(k)
                    .map_or(nothing.clone(), |p| self.inf.resolve(&p.ty))
            })
            .collect();
        inputs.push(match fs.params.len() > 1 {
            true => self.builtin_ty("Texture"),
            false => nothing,
        });
        Ty::Builtin("Pipeline", inputs)
    }

    /// `frame.draw(p, vertices, …)` must give the instances and the shared
    /// value `p`'s vertex fn takes.
    pub(crate) fn check_draw_args(&mut self, args: &[Arg], span: Span) {
        let Some(first) = args.first() else {
            return;
        };
        let typed = self.type_at(first.value.span).map(|t| self.inf.resolve(&t));
        let Some(Ty::Builtin("Pipeline", inputs)) = typed else {
            return;
        };
        let nothing = self.builtin_ty("NoInput");
        for (k, name) in [(1, "instances"), (2, "shared"), (3, "textures")] {
            let given = args
                .iter()
                .any(|a| a.name.as_ref().is_some_and(|n| n.name == name));
            if inputs.get(k).is_some_and(|t| *t != nothing) && !given {
                let t = self.show(&inputs[k]);
                let who = if k == 3 { "fragment" } else { "vertex" };
                self.error(span, format!("this pipeline's {who} fn takes a `{t}` too: give it as `{name}:` (spec 15.3)"));
            }
        }
    }
}

impl Checker<'_> {
    /// `std.vec`'s type `t` is (spec 14.28): `Vec4`, `Mat4` and the rest.
    pub(crate) fn std_vec_def(&self, def: DefId) -> bool {
        ["Vec2", "Vec3", "Vec4", "Mat4", "Quat"]
            .iter()
            .any(|n| self.res.std_type(n) == Some(def))
    }

    /// Four `Float32`s: a tuple of them, or `std.vec`'s `Vec4<Float32>`, as
    /// a corner's `position` or a pixel's color (spec 15.3, 14.28).
    pub(crate) fn four_floats(&self, t: &Ty) -> bool {
        match t {
            Ty::Adt(def, args) => {
                self.res.std_type("Vec4") == Some(*def)
                    && matches!(args[..], [Ty::Prim(Prim::Float32)])
            }
            t => four(t),
        }
    }
}
