// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A generic function or method given whole numbers with their own limits
//! (spec 2.1, 2.9): `twice(UInt<bits: 12>(7))`. Such values are kept in
//! an `Int`, so the copy made for `Int` would not check their limits.
//! Each set of such types a call gives its type parameters makes a copy
//! of its own, and inside it an operator on a `T` checks `T`'s limits
//! (`lower_whole.rs`), as the interpreter does.

use crate::ir::Type;
use crate::lower::Lowerer;
use kavach_ast::{self as ast, TypeKind};
use kavach_checker::{Prim, Ty};
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

/// Type parameter → the whole-number type with its own limits it stands for.
pub(crate) type Wholes = HashMap<DefId, Prim>;

impl<'a> Lowerer<'a> {
    /// The whole-number type with its own limits `t` is: the type itself,
    /// or a type parameter of the copy being made that stands for one.
    pub(crate) fn narrow(&self, t: &Ty) -> Option<Prim> {
        match t {
            Ty::Prim(p) if p.is_narrow() => Some(*p),
            Ty::Param(d) => self.wholes.get(d).copied(),
            _ => None,
        }
    }

    /// What call `call` of generic `f` gives its type parameters, from
    /// each argument's type, the result's and, for a method, the
    /// receiver's type arguments (`owner`'s parameters).
    pub(crate) fn call_wholes(
        &self,
        f: &ast::FnDecl,
        args: &[ast::Arg],
        call: &ast::Expr,
        owner: Option<DefId>,
    ) -> Wholes {
        let mut out = Wholes::new();
        let typed = f.params.iter().filter(|p| p.ty.is_some());
        for (i, p) in typed.enumerate() {
            let given = (args.iter())
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == p.name.name))
                .or_else(|| args.get(i).filter(|a| a.name.is_none()));
            if let (Some(a), Some(written)) = (given, &p.ty)
                && let Some(t) = self
                    .c
                    .expr_types
                    .get(&(a.value.span.start, a.value.span.end))
            {
                self.infer_wholes(written, t, &mut out);
            }
        }
        if let (Some(ret), Some(t)) = (
            &f.ret,
            self.c.expr_types.get(&(call.span.start, call.span.end)),
        ) {
            let t = match t {
                Ty::Result(ok, _) => ok,
                t => t,
            };
            self.infer_wholes(ret, t, &mut out);
        }
        let receiver = match &call.kind {
            ast::ExprKind::Call { callee, .. } => match &callee.kind {
                ast::ExprKind::Field { base, .. } => Some(base),
                _ => None,
            },
            _ => None,
        };
        if let (Some(owner), Some(base)) = (owner, receiver)
            && let Some(Ty::Adt(_, targs)) =
                self.c.expr_types.get(&(base.span.start, base.span.end))
        {
            for (d, t) in self.type_generics(owner).into_iter().zip(targs) {
                if let Some(p) = self.narrow(t) {
                    out.entry(d).or_insert(p);
                }
            }
        }
        out
    }

    /// Matches written type `written` against checked type `ty`, noting
    /// each type parameter that stands for a whole number with its own
    /// limits.
    fn infer_wholes(&self, written: &ast::TypeExpr, ty: &Ty, out: &mut Wholes) {
        let ty = match ty {
            Ty::Sized(x, ..) | Ty::Frozen(x, ..) => return self.infer_wholes(written, x, out),
            t => t,
        };
        match (&written.kind, ty) {
            (TypeKind::WithNums(base, _), _) => self.infer_wholes(base, ty, out),
            (TypeKind::Named { path, args }, _) => {
                let first = &path.segments[0];
                let Some(&d) = self.c.resolution.uses.get(&first.span.start) else {
                    return;
                };
                if self.c.resolution.defs[d].kind == DefKind::GenericParam {
                    if let Some(p) = self.narrow(ty) {
                        out.entry(d).or_insert(p);
                    }
                    return;
                }
                let inner: Vec<&Ty> = match ty {
                    Ty::Adt(_, ts) | Ty::Builtin(_, ts) | Ty::Tuple(ts) => ts.iter().collect(),
                    Ty::List(t) | Ty::Set(t) | Ty::Optional(t) => vec![t],
                    Ty::Map(k, v) => vec![k, v],
                    _ => return,
                };
                let types = args
                    .iter()
                    .filter(|a| !matches!(a.kind, TypeKind::Num { .. }));
                for (a, t) in types.zip(inner) {
                    self.infer_wholes(a, t, out);
                }
            }
            (TypeKind::List(item), Ty::List(t)) => self.infer_wholes(item, t, out),
            (TypeKind::Optional(inner), Ty::Optional(t)) => self.infer_wholes(inner, t, out),
            // A `T` passed where `T?` is expected.
            (TypeKind::Optional(inner), t) => self.infer_wholes(inner, t, out),
            (TypeKind::Map(k, v), Ty::Map(kt, vt)) => {
                self.infer_wholes(k, kt, out);
                self.infer_wholes(v, vt, out);
            }
            (TypeKind::Tuple(ws), Ty::Tuple(ts)) => {
                for (w, t) in ws.iter().zip(ts) {
                    self.infer_wholes(w, t, out);
                }
            }
            _ => {}
        }
    }

    /// What `wholes` adds to an instance's key, so each set of such types
    /// makes a copy of its own: nothing when there are none.
    pub(crate) fn whole_key(&mut self, params: &[DefId], wholes: &Wholes) -> Vec<Type> {
        if wholes.is_empty() {
            return Vec::new();
        }
        let mut key = Vec::new();
        for d in params {
            key.push(match wholes.get(d) {
                Some(p) => self.whole_type(*p),
                None => Type::Unit,
            });
        }
        key
    }
}
