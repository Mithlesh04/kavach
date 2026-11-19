// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `where T: Floating` on a method (spec 2.9): a bound the method adds to
//! its type's parameter. In the method's body the parameter has it; a
//! call is allowed only when the type given for the parameter meets it.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

impl Checker<'_> {
    /// Notes the `where` bounds of method `f` of the type `self_ty`. Only
    /// `Number` and `Floating` may be named there.
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
                match bound_name(b) {
                    Some("Floating") => found.push((param, true)),
                    Some("Number") => found.push((param, false)),
                    _ => self.error(
                        b.span,
                        "a `where` bound is `Number` or `Floating` (spec 2.9)",
                    ),
                }
            }
        }
        self.wheres.insert((*owner, f.name.name.clone()), found);
    }

    /// Gives `f`'s type parameters its `where` bounds while its body is
    /// checked; what they had before, for `undo_wheres`.
    pub(crate) fn apply_wheres(
        &mut self,
        f: &FnDecl,
        self_ty: Option<&Ty>,
    ) -> Vec<(DefId, Option<bool>)> {
        let Some(Ty::Adt(owner, _)) = self_ty else {
            return Vec::new();
        };
        let bounds = self.wheres.get(&(*owner, f.name.name.clone())).cloned();
        let mut before = Vec::new();
        for (param, floating) in bounds.unwrap_or_default() {
            let had = self.inf.numbers.get(&param).copied();
            before.push((param, had));
            let floating = floating || had == Some(true);
            self.inf.numbers.insert(param, floating);
        }
        before
    }

    /// Puts back what `apply_wheres` changed, last change first.
    pub(crate) fn undo_wheres(&mut self, before: Vec<(DefId, Option<bool>)>) {
        for (param, had) in before.into_iter().rev() {
            match had {
                Some(f) => self.inf.numbers.insert(param, f),
                None => self.inf.numbers.remove(&param),
            };
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
        for (param, floating) in bounds {
            let Some(t) = subst.get(&param) else { continue };
            let check = (param, floating, t.clone(), span, name.to_string());
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
    fn where_check(&mut self, check: (DefId, bool, Ty, Span, String)) {
        let (param, floating, t, span, method) = check;
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
