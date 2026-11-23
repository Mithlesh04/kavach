// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `where T: Floating` on a method (spec 2.9): a bound the method adds to
//! its type's parameter. In the method's body the parameter has it; a
//! call is allowed only when the type given for the parameter meets it.
//! The bound is `Number`, `Floating` or an operator's interface, as
//! `BitAnd` (spec 2.16), which lets the body use that operator.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

/// A bound a `where` names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Bound {
    Number,
    Floating,
    /// An operator's interface, as the method it gives (`bit_and`).
    Op(&'static str),
}

/// What `apply_wheres` changed, for `undo_wheres`: each parameter's number
/// bound before, and the operator methods it added.
pub(crate) type Applied = (Vec<(DefId, Option<bool>)>, Vec<(DefId, &'static str)>);

impl Checker<'_> {
    /// Notes the `where` bounds of method `f` of the type `self_ty`:
    /// `Number`, `Floating` or an operator's interface.
    pub(crate) fn note_wheres(&mut self, f: &FnDecl, self_ty: Option<&Ty>) {
        if f.where_bounds.is_empty() {
            return;
        }
        let Some(Ty::Adt(owner, params)) = self_ty else {
            let span = f.where_bounds[0].name.span;
            self.error(span, "only a method of a generic type has `where`: a function writes its bounds in `<T: …>` (spec 2.9)");
            return;
        };
        let mut found = Vec::new();
        for w in &f.where_bounds {
            let param = params.iter().find_map(|p| match p {
                Ty::Param(d) if self.res.defs[*d].name == w.name.name => Some(*d),
                _ => None,
            });
            let Some(param) = param else {
                let message = format!(
                    "`{}` is not a parameter of this type: `where` bounds the type's own parameters (spec 2.9)",
                    w.name.name
                );
                self.error(w.name.span, message);
                continue;
            };
            for b in &w.bounds {
                let name = bound_name(b);
                let bound = match name {
                    Some("Floating") => Some(Bound::Floating),
                    Some("Number") => Some(Bound::Number),
                    _ => name.and_then(crate::op_bounds::bound_method).map(Bound::Op),
                };
                match bound {
                    Some(bound) => found.push((param, bound)),
                    None => self.error(
                        b.span,
                        "a `where` bound is `Number`, `Floating` or an operator's interface, as `BitAnd` (spec 2.9)",
                    ),
                }
            }
        }
        self.wheres.insert((*owner, f.name.name.clone()), found);
    }

    /// Gives `f`'s type parameters its `where` bounds while its body is
    /// checked; what they had before, for `undo_wheres`.
    pub(crate) fn apply_wheres(&mut self, f: &FnDecl, self_ty: Option<&Ty>) -> Applied {
        let Some(Ty::Adt(owner, _)) = self_ty else {
            return (Vec::new(), Vec::new());
        };
        let bounds = self.wheres.get(&(*owner, f.name.name.clone())).cloned();
        let (mut before, mut ops) = (Vec::new(), Vec::new());
        for (param, bound) in bounds.unwrap_or_default() {
            if let Bound::Op(m) = bound {
                self.param_ops.entry(param).or_default().push(m);
                ops.push((param, m));
                continue;
            }
            let had = self.inf.numbers.get(&param).copied();
            before.push((param, had));
            let floating = bound == Bound::Floating || had == Some(true);
            self.inf.numbers.insert(param, floating);
        }
        (before, ops)
    }

    /// Puts back what `apply_wheres` changed, last change first.
    pub(crate) fn undo_wheres(&mut self, applied: Applied) {
        let (before, ops) = applied;
        for (param, had) in before.into_iter().rev() {
            match had {
                Some(f) => self.inf.numbers.insert(param, f),
                None => self.inf.numbers.remove(&param),
            };
        }
        for (param, m) in ops.into_iter().rev() {
            if let Some(ms) = self.param_ops.get_mut(&param)
                && let Some(i) = ms.iter().rposition(|x| *x == m)
            {
                ms.remove(i);
            }
        }
    }

    /// At a call of method `name` of type `owner`, with the type's
    /// parameters given by `subst`: notes each `where` bound to check.
    pub(crate) fn check_wheres(
        &mut self,
        owner: DefId,
        name: &str,
        subst: &HashMap<DefId, Ty>,
        span: Span,
    ) {
        let Some(bounds) = self.wheres.get(&(owner, name.to_string())).cloned() else {
            return;
        };
        for (param, bound) in bounds {
            let Some(t) = subst.get(&param) else { continue };
            let check = (param, bound, t.clone(), span, name.to_string());
            // A type parameter's bounds are those where the call is
            // (another `where` method's, say), so it is checked now.
            if matches!(self.inf.resolve(t), Ty::Param(_)) {
                self.where_check(check);
            } else {
                self.where_checks.push(check);
            }
        }
    }

    /// The `where` bounds noted at calls, now literals' types are known.
    pub(crate) fn wheres_now(&mut self) {
        for check in std::mem::take(&mut self.where_checks) {
            self.where_check(check);
        }
    }

    /// Whether a `where` bound noted at a call is met: an error if not.
    fn where_check(&mut self, check: (DefId, Bound, Ty, Span, String)) {
        let (param, bound, t, span, method) = check;
        if let Bound::Op(m) = bound {
            // A literal with no type of its own is an `Int` or a `Float`.
            if let Ty::Var(v) = self.inf.resolve(&t) {
                match self.inf.kind(v) {
                    crate::infer::VarKind::IntLiteral => self.inf.unify(&t, &Ty::Prim(Prim::Int)),
                    crate::infer::VarKind::FloatLiteral => {
                        self.inf.unify(&t, &Ty::Prim(Prim::Float))
                    }
                    _ => true,
                };
            }
            if self.has_operator(&t, m) == Some(false) {
                let (name, shown) = (self.res.defs[param].name.clone(), self.show(&t));
                let iface = crate::op_bounds::bound_interface(m);
                let message = format!(
                    "`{method}` needs `{name}: {iface}`, and `{name}` is `{shown}` here (spec 2.9)"
                );
                self.error(span, message);
            }
            return;
        }
        let floating = bound == Bound::Floating;
        let mut t = self.inf.resolve(&t);
        // A literal given where `Floating` is needed is a `Float`, as
        // for a function's bound (`numbers.rs`).
        if let Ty::Var(v) = t
            && floating
            && self.inf.kind(v) != crate::infer::VarKind::Any
        {
            self.inf.unify(&t, &Ty::Prim(Prim::Float));
            t = self.inf.resolve(&t);
        }
        let meets = match &t {
            Ty::Prim(p) if floating => matches!(p, Prim::Float | Prim::Float32),
            Ty::Prim(p) => p.is_numeric(),
            Ty::Param(_) => self.number_param(&t, floating),
            Ty::Var(_) | Ty::Error | Ty::Never => true,
            _ => false,
        };
        if !meets {
            let (name, shown) = (self.res.defs[param].name.clone(), self.show(&t));
            let bound = if floating { "Floating" } else { "Number" };
            let message = format!(
                "`{method}` needs `{name}: {bound}`, and `{name}` is `{shown}` here (spec 2.9)"
            );
            self.error(span, message);
        }
    }
}

/// The name of bound `b` when it is a plain name, as `Floating`.
fn bound_name(b: &TypeExpr) -> Option<&str> {
    match &b.kind {
        TypeKind::Named { path, args } if args.is_empty() && path.segments.len() == 1 => {
            Some(path.segments[0].name.as_str())
        }
        _ => None,
    }
}
