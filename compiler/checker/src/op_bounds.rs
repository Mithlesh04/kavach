// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Operator bounds (spec 2.16, 2.9, decision 0115): `T: Add` lets generic
//! code use `a + b` on two `T`s, which calls `add`: a program's own
//! type's method, or a number's, as every number type meets `Add`, `Sub`,
//! `Mul`, `Div`, `Rem`, `Neg` and `Comparable`. Text meets `Comparable`.
//! Whole numbers and `Bool` meet `BitAnd`, `BitOr` and `BitNot`.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

/// Each operator interface a bound can name, and its method.
const BOUNDS: [(&str, &str); 10] = [
    ("Add", "add"),
    ("Sub", "sub"),
    ("Mul", "mul"),
    ("Div", "div"),
    ("Rem", "rem"),
    ("Neg", "neg"),
    ("Comparable", "compare"),
    ("BitAnd", "bit_and"),
    ("BitOr", "bit_or"),
    ("BitNot", "bit_not"),
];

/// The method of the operator interface a bound names, as `bit_and` for
/// `BitAnd`.
pub(crate) fn bound_method(name: &str) -> Option<&'static str> {
    BOUNDS.iter().find(|(i, _)| *i == name).map(|(_, m)| *m)
}

/// The interface a bound's method comes from, as `BitAnd` for `bit_and`.
pub(crate) fn bound_interface(method: &str) -> &'static str {
    BOUNDS
        .iter()
        .find(|(_, m)| *m == method)
        .map_or("", |(i, _)| *i)
}

/// Whether `m` is a bit operator's method: whole numbers and `Bool`s
/// have them, other numbers don't (spec 2.16).
fn is_bit(m: &str) -> bool {
    matches!(m, "bit_and" | "bit_or" | "bit_not")
}

impl Checker<'_> {
    /// Notes `def`'s operator bounds from `g`, with no type arguments.
    pub(crate) fn note_op_bounds(&mut self, g: &GenericParam, def: DefId) {
        for b in &g.bounds {
            let TypeKind::Named { path, args } = &b.kind else {
                continue;
            };
            let name = path.segments.last().map(|s| s.name.as_str());
            if let Some((_, m)) = BOUNDS.iter().find(|(i, _)| Some(*i) == name)
                && args.is_empty()
                && path.segments.len() == 1
            {
                self.param_ops.entry(def).or_default().push(m);
            }
        }
    }

    /// `a op b` with `a` a `T` bounded by the operator's interface: noted
    /// as the method call it is. `None` when `T` has no such bound.
    pub(crate) fn param_binary(
        &mut self,
        method: &'static str,
        l: &Expr,
        r: &Expr,
        lt: &Ty,
    ) -> Option<Ty> {
        let Ty::Param(p) = self.inf.resolve(lt) else {
            return None;
        };
        if !self
            .param_ops
            .get(&p)
            .is_some_and(|ms| ms.contains(&method))
        {
            return None;
        }
        self.check_expect(r, lt);
        self.user_ops.insert((l.span.start, r.span.end), method);
        Some(if method == "compare" {
            Ty::Prim(Prim::Bool)
        } else {
            Ty::Param(p)
        })
    }

    /// `-a` with `a` a `T: Neg`, or `~a` with `a` a `T: BitNot`.
    pub(crate) fn param_unary(&mut self, method: &str, x: &Expr, t: &Ty) -> Option<Ty> {
        let Ty::Param(p) = self.inf.resolve(t) else {
            return None;
        };
        if !self
            .param_ops
            .get(&p)
            .is_some_and(|ms| ms.contains(&method))
        {
            return None;
        }
        self.user_negs.insert((x.span.start, x.span.end));
        Some(Ty::Param(p))
    }

    /// Whether type `t` has operator method `m`: a number has them all
    /// but the bit operators, which only whole numbers and `Bool` have,
    /// text `compare`, a program's own type the ones it implements, and a
    /// type parameter those its bounds give.
    pub(crate) fn has_operator(&self, t: &Ty, m: &str) -> Option<bool> {
        Some(match self.inf.resolve(t) {
            Ty::Prim(p) if p.is_numeric() => !is_bit(m) || p.is_int(),
            Ty::Prim(Prim::Bool) => is_bit(m),
            Ty::Prim(Prim::String) => m == "compare",
            Ty::Adt(def, _) => self.operators.contains_key(&(def, m)),
            // A number type has every arithmetic operator, so a `Number`
            // `T` does too.
            Ty::Param(q) => {
                self.param_ops.get(&q).is_some_and(|ms| ms.contains(&m))
                    || (self.inf.numbers.contains_key(&q) && !is_bit(m))
            }
            Ty::Var(_) | Ty::Error | Ty::Never => return None,
            _ => false,
        })
    }

    /// Checks that `t`, given for type parameter `g`, has the operators
    /// `g`'s bounds ask for.
    pub(crate) fn op_bounds_now(&mut self, g: DefId, t: &Ty, span: Span) {
        let Some(methods) = self.param_ops.get(&g).cloned() else {
            return;
        };
        for m in methods {
            if self.has_operator(t, m) == Some(false) {
                let (shown, name) = (self.show(t), &self.res.defs[g].name);
                let iface = bound_interface(m);
                self.error(span, format!("`{shown}` doesn't implement `{iface}`, which `{name}` needs here (spec 2.16)"));
            }
        }
    }
}
