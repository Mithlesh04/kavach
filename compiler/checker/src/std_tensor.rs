// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What the checker knows of `std.tensor` (spec 14.33, decision 0136):
//!
//! - a tensor holds numbers or `Bool`s, so `Tensor<String>` is an error;
//! - `Tensor<U>(t)` converts each number, as `U(x)` converts one; the
//!   driver calls the tensor's own `converted<U>()` for it;
//! - a number given as an argument where a `Tensor<T>` is asked for is a
//!   tensor of shape `[]`: `tensor.where(t < 0.0, 0.0, t)`;
//! - NumPy's names for its methods are errors that name Kavach's.

use crate::checker::Checker;
use crate::infer::VarKind;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// `Tensor` of `std.tensor`, when the program has it.
    pub(crate) fn tensor_def(&self) -> Option<DefId> {
        self.res.std_type("Tensor")
    }

    /// Whether `t` is a number type, or a type parameter bound to one.
    fn is_number(&self, t: &Ty) -> bool {
        match self.inf.resolve(t) {
            Ty::Prim(p) => p.is_numeric(),
            t @ Ty::Param(_) => self.number_param(&t, false),
            _ => false,
        }
    }

    /// `Tensor<U>(t)`: the converted tensor's type, or `None` when the call
    /// is not one (a tensor made from its fields).
    pub(crate) fn tensor_conversion(
        &mut self,
        def: DefId,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
    ) -> Option<Ty> {
        if self.tensor_def() != Some(def) {
            return None;
        }
        let ([a], [g]) = (args, generics) else {
            return None;
        };
        if a.name.is_some() {
            return None;
        }
        let to = self.lower(g, None);
        let got = self.value(&a.value, None);
        let from = match self.inf.resolve(&got) {
            Ty::Adt(d, targs) if d == def => targs.first().cloned().unwrap_or(Ty::Error),
            Ty::Error => return Some(Ty::Error),
            other => {
                let shown = self.show(&other);
                self.error(
                    a.value.span,
                    format!(
                        "`Tensor<U>(t)` converts a tensor's numbers, and this is {} (spec 14.33)",
                        crate::op_impls::a_shown(&shown)
                    ),
                );
                return Some(Ty::Error);
            }
        };
        for t in [&from, &to] {
            if !self.is_number(t) && !matches!(self.inf.resolve(t), Ty::Var(_) | Ty::Error) {
                let shown = self.show(t);
                self.error(span, format!("`Tensor<U>(t)` converts numbers, as `U(x)` does, and `{shown}` is not a number type (`T: Number`, spec 14.33)"));
            }
        }
        self.tensor_casts.insert((span.start, span.end));
        Some(Ty::Adt(def, vec![to]))
    }

    /// A number given where a `Tensor<T>` is asked for, as an argument: a
    /// tensor of shape `[]` holding it. Whether it was one.
    pub(crate) fn number_as_tensor(&mut self, arg: &Expr, actual: &Ty, want: &Ty) -> bool {
        let Some(def) = self.tensor_def() else {
            return false;
        };
        let Ty::Adt(d, targs) = self.inf.resolve(want) else {
            return false;
        };
        let item = match (d == def, targs.first()) {
            (true, Some(item)) => item.clone(),
            _ => return false,
        };
        let a = self.inf.resolve(actual);
        let plain = match &a {
            Ty::Prim(_) | Ty::Param(_) => true,
            Ty::Var(v) => self.inf.kind(*v) != VarKind::Any,
            _ => false,
        };
        if !plain {
            return false;
        }
        let before = self.inf.clone();
        if !self.inf.unify(&a, &item) {
            self.inf = before;
            return false;
        }
        self.as_tensors
            .insert((arg.span.start, arg.span.end), Ty::Adt(def, vec![item]));
        true
    }

    /// Each `Tensor<X>` the program uses holds numbers or `Bool`s: one
    /// error for each other item type, where it is first seen.
    pub(crate) fn check_tensor_items(&mut self) {
        let Some(def) = self.tensor_def() else {
            return;
        };
        let mut seen: Vec<(Span, Ty)> = Vec::new();
        let mut found: Vec<(Span, Ty)> = Vec::new();
        for (span, t) in &self.expr_types {
            seen.clear();
            self.tensors_in(&self.inf.resolve(t), def, &mut seen);
            found.extend(seen.drain(..).map(|(_, x)| (*span, x)));
        }
        found.sort_by_key(|(s, _)| (s.start, s.end));
        let mut told: Vec<String> = Vec::new();
        for (span, item) in found {
            let shown = self.show(&item);
            if told.contains(&shown) {
                continue;
            }
            told.push(shown.clone());
            self.error(span, format!("a tensor holds numbers or `Bool`s, so `Tensor<{shown}>` is not a type: its item type is a number type (`T: Number`) or `Bool` (spec 14.33)"));
        }
    }

    /// The item types of the tensors in `t` that are neither numbers nor
    /// `Bool`.
    fn tensors_in(&self, t: &Ty, def: DefId, out: &mut Vec<(Span, Ty)>) {
        match t {
            Ty::Adt(d, args) => {
                if *d == def
                    && let Some(item) = args.first()
                {
                    let item = self.inf.resolve(item);
                    let fine = match &item {
                        Ty::Prim(p) => p.is_numeric() || *p == Prim::Bool,
                        Ty::Param(_) | Ty::Var(_) | Ty::Error | Ty::Never => true,
                        _ => false,
                    };
                    if !fine {
                        out.push((Span::new(0, 0), item));
                    }
                }
                for a in args {
                    self.tensors_in(&self.inf.resolve(a), def, out);
                }
            }
            Ty::List(x) | Ty::Optional(x) | Ty::Set(x) => {
                self.tensors_in(&self.inf.resolve(x), def, out)
            }
            Ty::Tuple(xs) => xs
                .iter()
                .for_each(|x| self.tensors_in(&self.inf.resolve(x), def, out)),
            _ => {}
        }
    }

    /// `t.sum(axis: -1)`: a written negative axis is an error, as
    /// `xs[-1]` is (spec 14.33, 2.19).
    pub(crate) fn check_written_axis(&mut self, def: DefId, args: &[Arg]) {
        if self.tensor_def() != Some(def) {
            return;
        }
        for a in args {
            if a.name.as_ref().is_some_and(|n| n.name == "axis")
                && let ExprKind::Unary(UnaryOp::Neg, x) = &a.value.kind
                && matches!(x.kind, ExprKind::Int(_))
            {
                self.error(a.value.span, "axes count from 0, and a written negative axis is not one: the last axis is `t.dims() - 1` (spec 14.33)");
            }
        }
    }

    /// Kavach's name for a NumPy name used on a tensor (spec 14.33).
    pub(crate) fn numpy_name(&self, recv: &Ty, name: &str) -> Option<&'static str> {
        let Ty::Adt(d, _) = self.inf.resolve(recv) else {
            return None;
        };
        if Some(d) != self.tensor_def() {
            return None;
        }
        Some(match name {
            "ndim" | "rank" => "dims()",
            "size" => "len()",
            "clip" => "clamp(lo, hi)",
            "astype" => "Tensor<U>(t)",
            "T" => "transpose()",
            "dot" => "a @ b",
            "tolist" => "items",
            "scale" => "t * k",
            "product" => "matmul(b, fast: true)",
            _ => return None,
        })
    }
}
