// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Type inference: type variables, unification and substitution.

use crate::ty::{Prim, Ty};
use kavach_resolver::DefId;
use std::collections::HashMap;

/// What an unknown type may still become. Number literals take their type
/// from context (spec 1.7): `1` can be any number type, `0.1` a `Float`
/// or a `Decimal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VarKind {
    Any,
    IntLiteral,
    FloatLiteral,
}

#[derive(Default)]
pub(crate) struct Infer {
    vars: Vec<(Option<Ty>, VarKind)>,
    /// The type parameters with a number bound (spec 2.9): `true` for
    /// `Floating`, `false` for `Number`. A literal may be of their type.
    pub(crate) numbers: HashMap<DefId, bool>,
}

impl Infer {
    pub(crate) fn fresh(&mut self, kind: VarKind) -> Ty {
        self.vars.push((None, kind));
        Ty::Var(self.vars.len() as u32 - 1)
    }

    pub(crate) fn kind(&self, var: u32) -> VarKind {
        self.vars[var as usize].1
    }

    /// Follows bound variables at the top level, and sees through a
    /// number in a collection's type (`[Int]<max: 2>` is a `[Int]` here).
    pub(crate) fn shallow(&self, ty: &Ty) -> Ty {
        let mut ty = self.shallow_keep(ty);
        while let Ty::Sized(base, ..) = ty {
            ty = self.shallow_keep(&base);
        }
        ty
    }

    /// Follows bound variables at the top level, keeping a collection's
    /// number (spec 2.9).
    pub(crate) fn shallow_keep(&self, ty: &Ty) -> Ty {
        let mut ty = ty.clone();
        while let Ty::Var(v) = ty {
            match &self.vars[v as usize].0 {
                Some(bound) => ty = bound.clone(),
                None => break,
            }
        }
        ty
    }

    /// Replaces every bound variable, all the way down, without the
    /// number of the collection's own type: its items keep theirs
    /// (`[[Int]<max: 2>]` stays a list of limited lists).
    pub(crate) fn resolve(&self, ty: &Ty) -> Ty {
        self.map(&self.shallow(ty), &|t| Some(self.resolve_keep(t)))
    }

    /// Replaces every bound variable, all the way down, without any
    /// collection's number: the types code generation sees.
    pub(crate) fn resolve_plain(&self, ty: &Ty) -> Ty {
        self.map(&self.shallow(ty), &|t| Some(self.resolve_plain(t)))
    }

    /// Replaces every bound variable, all the way down, keeping the
    /// numbers of collections' types.
    pub(crate) fn resolve_keep(&self, ty: &Ty) -> Ty {
        self.map(&self.shallow_keep(ty), &|t| Some(self.resolve_keep(t)))
    }

    /// Replaces generic parameters using `subst`.
    pub(crate) fn subst(&self, ty: &Ty, subst: &HashMap<DefId, Ty>) -> Ty {
        match self.shallow_keep(ty) {
            Ty::Param(p) => subst.get(&p).cloned().unwrap_or(Ty::Param(p)),
            other => self.map(&other, &|t| Some(self.subst(t, subst))),
        }
    }

    /// The generic parameters in `ty`, all the way down.
    pub(crate) fn params(&self, ty: &Ty) -> Vec<DefId> {
        let found = std::cell::RefCell::new(Vec::new());
        self.collect_params(&self.resolve_keep(ty), &found);
        found.into_inner()
    }

    fn collect_params(&self, ty: &Ty, found: &std::cell::RefCell<Vec<DefId>>) {
        if let Ty::Param(p) = ty {
            found.borrow_mut().push(*p);
            return;
        }
        self.map(ty, &|t| {
            self.collect_params(t, found);
            None
        });
    }

    /// Applies `f` to the direct children of `ty`.
    fn map(&self, ty: &Ty, f: &dyn Fn(&Ty) -> Option<Ty>) -> Ty {
        let g = |t: &Ty| f(t).unwrap_or_else(|| t.clone());
        let b = |t: &Ty| Box::new(g(t));
        match ty {
            Ty::List(t) => Ty::List(b(t)),
            Ty::Set(t) => Ty::Set(b(t)),
            Ty::Optional(t) => Ty::Optional(b(t)),
            Ty::Range(t) => Ty::Range(b(t)),
            Ty::Map(k, v) => Ty::Map(b(k), b(v)),
            Ty::Result(o, e) => Ty::Result(b(o), b(e)),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(g).collect()),
            Ty::Anon(fs) => Ty::Anon(fs.iter().map(|(n, t)| (n.clone(), g(t))).collect()),
            Ty::Fn(ps, r, u) => Ty::Fn(ps.iter().map(g).collect(), b(r), u.clone()),
            Ty::Adt(d, ts) => Ty::Adt(*d, ts.iter().map(g).collect()),
            Ty::Builtin(n, ts) => Ty::Builtin(n, ts.iter().map(g).collect()),
            Ty::Sized(t, k, n) => Ty::Sized(b(t), *k, b(n)),
            other => other.clone(),
        }
    }

    /// Makes `a` and `b` the same type if possible. Returns false if they
    /// can't be.
    pub(crate) fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let (a, b) = (self.shallow_keep(a), self.shallow_keep(b));
        match (&a, &b) {
            (Ty::Error, _) | (_, Ty::Error) | (Ty::Never, _) | (_, Ty::Never) => true,
            (Ty::Var(x), Ty::Var(y)) if x == y => true,
            (Ty::Var(x), Ty::Var(y)) => match merge(self.kind(*x), self.kind(*y)) {
                Some(kind) => {
                    self.vars[*y as usize].1 = kind;
                    self.vars[*x as usize].0 = Some(b.clone());
                    true
                }
                None => false,
            },
            (Ty::Var(x), t) | (t, Ty::Var(x)) => {
                if !self.fits(self.kind(*x), t) || self.occurs(*x, t) {
                    return false;
                }
                self.vars[*x as usize].0 = Some(t.clone());
                true
            }
            // `[Float]<len: 3>` and `[Float]<len: 2>` are two types; a
            // number on one side only is checked where a value is given
            // (`Checker::coerce`, spec 2.9).
            (Ty::Sized(x, k1, n1), Ty::Sized(y, k2, n2)) => {
                k1 == k2 && self.unify(x, y) && self.unify(n1, n2)
            }
            (Ty::Sized(x, ..), t) | (t, Ty::Sized(x, ..)) => self.unify(x, t),
            (Ty::Num(x), Ty::Num(y)) => x == y,
            (Ty::Prim(p), Ty::Prim(q)) => p == q,
            (Ty::Unit, Ty::Unit) => true,
            (Ty::List(x), Ty::List(y))
            | (Ty::Set(x), Ty::Set(y))
            | (Ty::Optional(x), Ty::Optional(y))
            | (Ty::Range(x), Ty::Range(y)) => self.unify(x, y),
            (Ty::Map(k1, v1), Ty::Map(k2, v2)) | (Ty::Result(k1, v1), Ty::Result(k2, v2)) => {
                self.unify(k1, k2) && self.unify(v1, v2)
            }
            (Ty::Tuple(xs), Ty::Tuple(ys)) => xs.len() == ys.len() && self.unify_all(xs, ys),
            (Ty::Anon(xs), Ty::Anon(ys)) => {
                xs.len() == ys.len()
                    && xs
                        .iter()
                        .zip(ys)
                        .all(|((n, x), (m, y))| n == m && self.unify(x, y))
            }
            (Ty::Fn(p1, r1, _), Ty::Fn(p2, r2, _)) => {
                p1.len() == p2.len() && self.unify_all(p1, p2) && self.unify(r1, r2)
            }
            (Ty::Adt(d1, a1), Ty::Adt(d2, a2)) => d1 == d2 && self.unify_args(a1, a2),
            (Ty::Builtin(n1, a1), Ty::Builtin(n2, a2)) => n1 == n2 && self.unify_args(a1, a2),
            (Ty::Param(x), Ty::Param(y)) => x == y,
            _ => false,
        }
    }

    fn unify_all(&mut self, xs: &[Ty], ys: &[Ty]) -> bool {
        xs.iter().zip(ys).all(|(x, y)| self.unify(x, y))
    }

    /// Type arguments left out (`Pair` instead of `Pair<Int, String>`) match anything.
    fn unify_args(&mut self, xs: &[Ty], ys: &[Ty]) -> bool {
        xs.is_empty() || ys.is_empty() || (xs.len() == ys.len() && self.unify_all(xs, ys))
    }

    /// Whether `var` appears inside `ty` (binding it would make an infinite type).
    fn occurs(&self, var: u32, ty: &Ty) -> bool {
        contains_var(&self.resolve(ty), var)
    }
}

fn contains_var(ty: &Ty, var: u32) -> bool {
    match ty {
        Ty::Var(v) => *v == var,
        Ty::List(t) | Ty::Set(t) | Ty::Optional(t) | Ty::Range(t) => contains_var(t, var),
        Ty::Sized(t, _, n) => contains_var(t, var) || contains_var(n, var),
        Ty::Map(a, b) | Ty::Result(a, b) => contains_var(a, var) || contains_var(b, var),
        Ty::Tuple(ts) | Ty::Adt(_, ts) | Ty::Builtin(_, ts) => {
            ts.iter().any(|t| contains_var(t, var))
        }
        Ty::Anon(fs) => fs.iter().any(|(_, t)| contains_var(t, var)),
        Ty::Fn(ps, r, _) => ps.iter().any(|t| contains_var(t, var)) || contains_var(r, var),
        _ => false,
    }
}

fn merge(a: VarKind, b: VarKind) -> Option<VarKind> {
    use VarKind::*;
    Some(match (a, b) {
        (Any, k) | (k, Any) => k,
        (IntLiteral, IntLiteral) => IntLiteral,
        (FloatLiteral, _) | (_, FloatLiteral) => FloatLiteral,
    })
}

impl Infer {
    /// Whether a variable of `kind` may become `ty`: a literal a number
    /// type, or a type parameter with a number bound (spec 2.9).
    fn fits(&self, kind: VarKind, ty: &Ty) -> bool {
        match (kind, ty) {
            (VarKind::IntLiteral, Ty::Param(p)) => self.numbers.contains_key(p),
            (VarKind::FloatLiteral, Ty::Param(p)) => self.numbers.get(p) == Some(&true),
            _ => fits(kind, ty),
        }
    }
}

fn fits(kind: VarKind, ty: &Ty) -> bool {
    match (kind, ty) {
        (VarKind::Any, _) => true,
        (VarKind::IntLiteral, Ty::Prim(p)) => p.is_numeric(),
        (VarKind::FloatLiteral, Ty::Prim(p)) => {
            matches!(p, Prim::Float | Prim::Float32 | Prim::Decimal)
        }
        _ => false,
    }
}
