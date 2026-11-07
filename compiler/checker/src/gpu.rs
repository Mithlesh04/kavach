//! `gpu fn` (spec 15): the rules that make one safe to run on the GPU,
//! and the `map` calls that may run there.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::collections::{HashMap, HashSet};

pub(crate) use crate::gpu_walk::Walk;

/// What a `gpu fn` takes and gives, and the only types inside one.
pub(crate) fn gpu_type(t: &Ty) -> bool {
    matches!(
        t,
        Ty::Prim(Prim::Float32 | Prim::Int32 | Prim::UInt32 | Prim::Bool)
    )
}

/// A call that may run on the GPU: its span, the list's item type, the
/// locals its lambda captures, and which method it is.
pub(crate) type GpuCandidate = ((u32, u32), Ty, Vec<DefId>, GpuCall);

/// A list method that may run on the GPU (spec 15.2).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum GpuCall {
    Map,
    /// `sum(f)` of whole numbers, which the GPU adds exactly.
    Sum,
    /// `min()` or `max()` of whole numbers.
    MinMax,
    /// `filter(f)` of a list of one of the four types.
    Filter,
}

/// Whether `t` is `Int32` or `UInt32`: a GPU `sum`, `min` or `max` gives
/// exactly what the CPU does in any order (decision 0055).
pub(crate) fn whole32(t: &Ty) -> bool {
    matches!(t, Ty::Prim(Prim::Int32 | Prim::UInt32))
}

/// Whether a `map` on the GPU can take items of type `t` (spec 15.2): one
/// of the four types, or a tuple or record of at most 8 of them, which a
/// list keeps flat.
pub(crate) fn gpu_item(c: &Checker<'_>, t: &Ty) -> bool {
    let fields: Vec<Ty> = match t {
        Ty::Tuple(ts) => ts.clone(),
        Ty::Anon(fs) => fs.iter().map(|(_, t)| t.clone()).collect(),
        Ty::Adt(def, args) if args.is_empty() => match c.types.get(def).map(|i| &i.shape) {
            Some(crate::checker::Shape::Record(fs)) => {
                fs.iter().map(|f| c.defaulted(&f.ty)).collect()
            }
            _ => return false,
        },
        t => return gpu_type(t),
    };
    (1..=8).contains(&fields.len()) && fields.iter().all(gpu_type)
}

pub(crate) const WHY: &str = "a `gpu fn` works with `Float32`, `Int32`, `UInt32` and `Bool`, and records and tuples of 1 to 8 of them (spec 15.1)";

impl Checker<'_> {
    /// Notes the `gpu fn`s, before bodies are checked, so `map` calls
    /// know them.
    pub(crate) fn collect_gpu_fns(&mut self, file: &File) {
        for item in &file.items {
            if let ItemKind::Fn(f) = &item.kind
                && let (true, Some(&def)) = (f.is_gpu, self.decl.get(&f.name.span.start))
            {
                self.gpu_fns.insert(def);
            }
        }
    }

    /// After inference: each `gpu fn`'s signature, body and calls.
    pub(crate) fn check_gpu_fns(&mut self, file: &File) {
        self.gpu_literal_defaults(file);
        let types: HashMap<(u32, u32), Ty> = self
            .expr_types
            .iter()
            .map(|(s, t)| ((s.start, s.end), self.defaulted(t)))
            .collect();
        let fits = self.gpu_fits(file, &types);
        let mut calls: HashMap<DefId, Vec<DefId>> = HashMap::new();
        for item in &file.items {
            let ItemKind::Fn(f) = &item.kind else {
                continue;
            };
            let Some(&def) = self.decl.get(&f.name.span.start).filter(|_| f.is_gpu) else {
                continue;
            };
            self.gpu_signature(f, def);
            let mut walk = Walk::new(&types, &fits);
            if let Some(body) = &f.body {
                walk.block(self, body);
            }
            for (span, message) in walk.errors {
                self.error(span, message);
            }
            calls.insert(def, walk.calls);
        }
        self.gpu_recursion(file, &calls);
    }

    /// Inside a `gpu fn`, a number with no other type is an `Int32` or a
    /// `Float32`, not an `Int` or a `Float` (spec 15.1).
    fn gpu_literal_defaults(&mut self, file: &File) {
        let bodies: Vec<Span> = file
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                ItemKind::Fn(f) if f.is_gpu => f.body.as_ref().map(|b| b.span),
                _ => None,
            })
            .collect();
        let inside = |s: &Span| bodies.iter().any(|b| b.start <= s.start && s.end <= b.end);
        let open: Vec<Ty> = self
            .expr_types
            .iter()
            .filter(|(s, _)| inside(s))
            .map(|(_, t)| t.clone())
            .collect();
        for t in open {
            let t = match self.inf.resolve(&t) {
                Ty::Range(item) => *item,
                t => t,
            };
            if let Ty::Var(v) = self.inf.shallow(&t) {
                let prim = match self.inf.kind(v) {
                    crate::infer::VarKind::IntLiteral => Prim::Int32,
                    crate::infer::VarKind::FloatLiteral => Prim::Float32,
                    crate::infer::VarKind::Any => continue,
                };
                self.inf.unify(&t, &Ty::Prim(prim));
            }
        }
    }

    fn gpu_signature(&mut self, f: &FnDecl, def: DefId) {
        if !f.generics.is_empty() || f.errors.is_some() || !f.uses.is_empty() {
            self.error(
                f.name.span,
                "a `gpu fn` can't be generic, fail (`!`) or use capabilities: on the GPU it runs alone, item by item (spec 15.1)",
            );
        }
        let Some(sig) = self.fns.get(&def).cloned() else {
            return;
        };
        let typed = f.params.iter().filter(|p| p.name.name != "self");
        for (p, info) in typed.zip(&sig.params) {
            let t = self.inf.resolve(&info.ty);
            if !gpu_item(self, &t) {
                let shown = self.show(&t);
                self.error(
                    p.name.span,
                    format!("`{}` is a `{shown}`, and {WHY}", p.name.name),
                );
            }
        }
        // A `gpu fn` gives its result back, called from `map` or from
        // another `gpu fn` alike (spec 15.1).
        let inout = f.params.iter().find(|p| p.is_inout);
        if let Some(p) = inout {
            self.error(
                p.name.span,
                format!(
                    "a `gpu fn` can't take `inout` parameters: give the new value back instead, as `gpu fn {}({}: Float32) -> Float32` (spec 15.1)",
                    f.name.name, p.name.name
                ),
            );
        }
        let ret = self.inf.resolve(&sig.ret);
        if !gpu_item(self, &ret) && inout.is_none() {
            let shown = self.show(&ret);
            self.error(
                f.name.span,
                format!("`{}` gives a `{shown}`, and {WHY}", f.name.name),
            );
        }
    }

    /// A `gpu fn` that reaches itself through calls: GPUs have no stack.
    fn gpu_recursion(&mut self, file: &File, calls: &HashMap<DefId, Vec<DefId>>) {
        for item in &file.items {
            let ItemKind::Fn(f) = &item.kind else {
                continue;
            };
            let Some(&start) = self.decl.get(&f.name.span.start).filter(|_| f.is_gpu) else {
                continue;
            };
            let (mut seen, mut todo) = (
                HashSet::new(),
                calls.get(&start).cloned().unwrap_or_default(),
            );
            while let Some(d) = todo.pop() {
                if d == start {
                    self.error(
                        f.name.span,
                        format!("`{}` calls itself, directly or through other `gpu fn`s: a GPU has no call stack (spec 15.1)", f.name.name),
                    );
                    break;
                }
                if seen.insert(d) {
                    todo.extend(calls.get(&d).cloned().unwrap_or_default());
                }
            }
        }
    }

    /// `items.map(f)` with `items` holding `item`s (spec 15.2): a
    /// candidate for the GPU when `f` is a `gpu fn` or a lambda that
    /// calls one; settled once the types are known (`gpu_maps`). A `sum`'s
    /// lambda may be any that fits (`x => x`), and `min` and `max` take
    /// none: they run on the GPU only when the list is there already.
    pub(crate) fn note_gpu_map(&mut self, call: Span, item: &Ty, args: &[Arg], kind: GpuCall) {
        if kind == GpuCall::MinMax {
            if args.is_empty() {
                self.gpu_candidates
                    .push(((call.start, call.end), item.clone(), Vec::new(), kind));
            }
            return;
        }
        let [arg] = args else { return };
        let mut captures = Vec::new();
        let ok = match &arg.value.kind {
            ExprKind::Name(id) => self
                .res
                .uses
                .get(&id.span.start)
                .is_some_and(|d| self.gpu_fns.contains(d)),
            ExprKind::Lambda { params, body } if params.len() == 1 => {
                let param = self.decl.get(&params[0].name.span.start).copied();
                let mut calls_gpu = false;
                self.lambda_fits(body, param, &mut captures, &mut calls_gpu)
                    && (calls_gpu || matches!(kind, GpuCall::Sum | GpuCall::Filter))
            }
            _ => false,
        };
        if ok {
            self.gpu_candidates
                .push(((call.start, call.end), item.clone(), captures, kind));
        }
    }

    /// Whether lambda body `e` is only `param`, numbers, captured names
    /// and calls to `gpu fn`s, joined by operators.
    fn lambda_fits(
        &self,
        e: &Expr,
        param: Option<DefId>,
        captures: &mut Vec<DefId>,
        calls_gpu: &mut bool,
    ) -> bool {
        match &e.kind {
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) => true,
            ExprKind::Name(id) => match self.res.uses.get(&id.span.start).copied() {
                Some(d) if Some(d) == param => true,
                Some(d)
                    if matches!(
                        self.res.defs[d].kind,
                        DefKind::Local { .. } | DefKind::Param
                    ) =>
                {
                    captures.push(d);
                    true
                }
                _ => false,
            },
            ExprKind::Unary(UnaryOp::Neg | UnaryOp::Not, x) => {
                self.lambda_fits(x, param, captures, calls_gpu)
            }
            // A field of the item: `p.x`, or `p.0` of a tuple.
            ExprKind::Field {
                base,
                optional: false,
                ..
            }
            | ExprKind::TupleField { base, .. } => matches!(&base.kind, ExprKind::Name(id)
                if param.is_some() && self.res.uses.get(&id.span.start).copied() == param),
            ExprKind::Binary(op, l, r) => {
                !matches!(
                    op,
                    BinaryOp::Default | BinaryOp::Range | BinaryOp::RangeInclusive
                ) && self.lambda_fits(l, param, captures, calls_gpu)
                    && self.lambda_fits(r, param, captures, calls_gpu)
            }
            ExprKind::Call { callee, args, .. } => {
                let ExprKind::Name(id) = &callee.kind else {
                    return false;
                };
                let gpu = self
                    .res
                    .uses
                    .get(&id.span.start)
                    .is_some_and(|d| self.gpu_fns.contains(d));
                *calls_gpu |= gpu;
                gpu && args.iter().all(|a| {
                    a.name.is_none() && self.lambda_fits(&a.value, param, captures, calls_gpu)
                })
            }
            _ => false,
        }
    }
}
