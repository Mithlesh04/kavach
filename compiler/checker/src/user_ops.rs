// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Operators for a program's own types (spec 2.16, decision 0109): a type
//! that implements `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `MatMul` or
//! `Comparable` gets the operator, which calls its method. Each use is
//! noted, and the driver turns it into that method call.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

/// Each operator interface and its method.
const OPERATORS: &[(&str, &str)] = &[
    ("Add", "add"),
    ("Sub", "sub"),
    ("Mul", "mul"),
    ("Div", "div"),
    ("Rem", "rem"),
    ("Neg", "neg"),
    ("MatMul", "matmul"),
    ("Comparable", "compare"),
];

/// The method an operator calls on a program's own type.
fn method_of(op: BinaryOp) -> Option<&'static str> {
    use BinaryOp::*;
    Some(match op {
        Add => "add",
        Sub => "sub",
        Mul => "mul",
        Div => "div",
        Rem => "rem",
        MatMul => "matmul",
        Lt | Le | Gt | Ge => "compare",
        _ => return None,
    })
}

/// The interface that gives a method's operator.
fn interface_of(method: &str) -> &'static str {
    OPERATORS
        .iter()
        .find(|(_, m)| *m == method)
        .map_or("", |(i, _)| i)
}

impl Checker<'_> {
    /// `type Vec2: Mul<Float>`: notes that `owner` has the operator, taking
    /// the other operand's type and giving the result's. Whether `i` names
    /// an operator interface at all.
    pub(crate) fn note_operator(
        &mut self,
        owner: Option<DefId>,
        i: &TypeExpr,
        self_ty: &Ty,
    ) -> bool {
        if let Some(noted) = self.note_indexer(owner, i, self_ty) {
            return noted;
        }
        let TypeKind::Named { path, args } = &i.kind else {
            return false;
        };
        let Some(name) = path.segments.last().map(|s| s.name.as_str()) else {
            return false;
        };
        let Some(&(iface, method)) = OPERATORS.iter().find(|(n, _)| *n == name) else {
            return false;
        };
        let Some(def) = owner else {
            let shown = self.show(self_ty);
            self.error(i.span, format!("`{shown}` keeps its own operators: a built-in type can't implement `{iface}` (spec 2.16)"));
            return true;
        };
        let given: Vec<Ty> = args.iter().map(|a| self.lower(a, Some(self_ty))).collect();
        let me = self_ty.clone();
        let (other, out) = match (method, given.as_slice()) {
            ("neg", []) => (Ty::Unit, me),
            ("neg", [out]) => (Ty::Unit, out.clone()),
            ("compare", []) => (me, Ty::Prim(Prim::Int)),
            (_, []) => (me.clone(), me),
            (_, [other]) => (other.clone(), me),
            (_, [other, out]) if method != "neg" && method != "compare" => {
                (other.clone(), out.clone())
            }
            _ => {
                self.error(i.span, format!("`{iface}` takes too many type arguments: `{iface}`, `{iface}<Other>` or `{iface}<Other, Result>` (spec 2.16)"));
                return true;
            }
        };
        self.operators.insert((def, method), (other, out, i.span));
        true
    }

    /// Each operator method is there, with the operator's types, pure and
    /// unable to fail (spec 2.16).
    pub(crate) fn check_operator_methods(&mut self) {
        let mut wanted: Vec<_> = self
            .operators
            .iter()
            .filter(|((_, m), _)| !interface_of(m).is_empty())
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        wanted.sort_by_key(|((def, m), _)| (*def, *m));
        for ((def, method), (other, out, span)) in wanted {
            let ty = self.res.defs[def].name.clone();
            let iface = interface_of(method);
            let (shown_other, shown_out) = (self.show(&other), self.show(&out));
            let want = if method == "neg" {
                format!("fn neg(self) -> {shown_out}")
            } else {
                format!("fn {method}(self, other: {shown_other}) -> {shown_out}")
            };
            let Some(sig) = self.methods.get(&def).and_then(|m| m.get(method)).cloned() else {
                self.error(
                    span,
                    format!("`{ty}` says it implements `{iface}`, so it needs the method `{want}`"),
                );
                continue;
            };
            let params: Vec<Ty> = sig.params.iter().map(|p| self.inf.resolve(&p.ty)).collect();
            let fits = sig.has_self
                && match method {
                    "neg" => params.is_empty(),
                    _ => params.len() == 1 && params[0] == self.inf.resolve(&other),
                }
                && self.inf.resolve(&sig.ret) == self.inf.resolve(&out);
            if !fits {
                self.error(
                    span,
                    format!("`{ty}.{method}` must be `{want}` for `{iface}` (spec 2.16)"),
                );
                continue;
            }
            let uses = sig
                .def
                .and_then(|d| self.fn_uses.get(&d))
                .is_some_and(|u| !u.is_empty());
            if sig.errors.is_some() || !sig.uses.is_empty() || uses {
                self.error(span, format!("`{ty}.{method}` gives an operator, so it can't use capabilities (`uses`) or fail (`!`): an operator never touches the outside world or fails (spec 2.16)"));
            }
        }
    }

    /// `a + b` (or `a < b`) with `a` of a program's own type: checks `b`,
    /// notes the call, and gives the result's type. `None` when `a` is not
    /// a program's own type.
    pub(crate) fn user_binary(&mut self, op: BinaryOp, l: &Expr, r: &Expr, lt: &Ty) -> Option<Ty> {
        // `a + b` on a `T: Add` (spec 2.16).
        if let Some(m) = method_of(op)
            && let Some(t) = self.param_binary(m, l, r, lt)
        {
            return Some(t);
        }
        let Ty::Adt(def, args) = self.inf.resolve(lt) else {
            return None;
        };
        let method = method_of(op)?;
        let Some((other, out, _)) = self.operators.get(&(def, method)).cloned() else {
            let (ty, iface) = (self.show(lt), interface_of(method));
            self.error(
                l.span,
                format!(
                    "`{ty}` has no `{}`: give it one by implementing `{iface}` (spec 2.16)",
                    op.as_str()
                ),
            );
            self.value(r, None);
            return Some(Ty::Error);
        };
        let subst = self.subst_of(def, &args);
        let (other, out) = (self.inf.subst(&other, &subst), self.inf.subst(&out, &subst));
        self.check_expect(r, &other);
        self.user_ops.insert((l.span.start, r.span.end), method);
        Some(if method == "compare" {
            Ty::Prim(Prim::Bool)
        } else {
            out
        })
    }

    /// `-a` with `a` of a program's own type.
    pub(crate) fn user_neg(&mut self, x: &Expr, t: &Ty) -> Option<Ty> {
        if let Some(t) = self.param_neg(x, t) {
            return Some(t);
        }
        let Ty::Adt(def, args) = self.inf.resolve(t) else {
            return None;
        };
        let Some((_, out, _)) = self.operators.get(&(def, "neg")).cloned() else {
            let ty = self.show(t);
            self.error(
                x.span,
                format!("`{ty}` has no `-`: give it one by implementing `Neg` (spec 2.16)"),
            );
            return Some(Ty::Error);
        };
        let subst = self.subst_of(def, &args);
        self.user_negs.insert((x.span.start, x.span.end));
        Some(self.inf.subst(&out, &subst))
    }

    /// `c += a` with `c` of a program's own type: `c = c.add(a)`, so the
    /// result must be `c`'s own type. Whether it was one.
    pub(crate) fn user_assign(
        &mut self,
        op: BinaryOp,
        target: &Expr,
        value: &Expr,
        t: &Ty,
    ) -> bool {
        let Some(result) = self.user_binary(op, target, value, t) else {
            return false;
        };
        if !matches!(result, Ty::Error) && self.inf.resolve(&result) != self.inf.resolve(t) {
            let (got, want) = (self.show(&result), self.show(t));
            self.error(target.span, format!("`{}=` gives a `{got}` here, which can't be stored in this `{want}`: write `x = x {} v` into a new variable", op.as_str(), op.as_str()));
        }
        if let Some(m) = self.user_ops.remove(&(target.span.start, value.span.end)) {
            self.user_assigns
                .insert((target.span.start, value.span.end), m);
        }
        true
    }

    /// The right of `n op r` with `n` a number: as `check_expect`, but
    /// `2.0 * v` with `v` of a program's own type says to swap them.
    pub(crate) fn number_then_own(&mut self, op: BinaryOp, r: &Expr, lt: &Ty) {
        let rt = self.value(r, Some(lt));
        self.fitting = Some(self.made(r));
        let fits = self.coerce(&rt, lt);
        self.fitting = None;
        if fits {
            return;
        }
        let own = match (self.inf.resolve(&rt), method_of(op)) {
            (Ty::Adt(def, _), Some(m)) => self.operators.contains_key(&(def, m)),
            _ => false,
        };
        if own {
            let (ty, op) = (self.show(&rt), op.as_str());
            self.error(r.span, format!("`{ty}` goes on the left of `{op}`: write `x {op} n`, not `n {op} x` (spec 2.16)"));
        } else {
            self.mismatch(r, lt, &rt);
        }
    }
}
