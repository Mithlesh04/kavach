// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whether a value of one type fits where another is needed: a `T`
//! where a `T?` is, and a type where an interface it implements is.

use crate::checker::{Checker, impl_key_adt};
use crate::ty::Ty;
use std::collections::HashMap;

impl Checker<'_> {
    /// Whether a value of type `actual` fits where `expected` is needed.
    /// A `T` fits where a `T?` is expected (spec 2.7).
    pub(crate) fn coerce(&mut self, actual: &Ty, expected: &Ty) -> bool {
        // Numbers in types (spec 2.9).
        if let Some(fits) = self.coerce_sized(actual, expected) {
            return fits;
        }
        let (a, e) = (self.inf.shallow(actual), self.inf.shallow(expected));
        // A value of a type that implements the interface fits where the
        // interface is expected (spec 2.15).
        if self.implements_interface(&a, &e) {
            return self.iface_args_fit(&a, &e);
        }
        // A number literal fits an interface its own type implements:
        // `4` is an `Int`, `2.5` a `Float`.
        if let Ty::Var(v) = &a {
            let own = match self.inf.kind(*v) {
                crate::infer::VarKind::IntLiteral => Some(crate::Prim::Int),
                crate::infer::VarKind::FloatLiteral => Some(crate::Prim::Float),
                crate::infer::VarKind::Any => None,
            };
            if let Some(p) = own.map(Ty::Prim)
                && self.implements_interface(&p, &e)
            {
                self.inf.unify(&a, &p);
                return self.iface_args_fit(&p, &e);
            }
        }
        if let (Ty::Optional(x), Ty::Optional(y)) = (&a, &e)
            && self.implements_interface(x, y)
        {
            return self.iface_args_fit(x, y);
        }
        if let (Ty::Fn(..), Ty::Fn(..)) = (&a, &e) {
            return self.fn_fits(&a, &e);
        }
        if let Ty::Optional(inner) = &e {
            // A number literal is never itself optional, so it fills the inner type.
            let literal = matches!(a, Ty::Var(v) if self.inf.kind(v) != crate::infer::VarKind::Any);
            if literal || !matches!(a, Ty::Optional(_) | Ty::Var(_)) {
                return self.inf.unify(&a, inner);
            }
        }
        self.inf.unify(&a, &e)
    }

    /// Whether `actual` is a type that implements the interface
    /// `expected` names (spec 2.15, decision 0029).
    pub(crate) fn implements_interface(&self, actual: &Ty, expected: &Ty) -> bool {
        let Ty::Adt(iface, _) = expected else {
            return false;
        };
        if self.res.defs[*iface].kind != kavach_resolver::DefKind::Interface {
            return false;
        }
        match actual {
            Ty::Adt(d, _) => {
                d != iface && self.implements.get(d).is_some_and(|s| s.contains(iface))
            }
            Ty::Prim(p) => self
                .ext_implements
                .get(p.name())
                .is_some_and(|s| s.contains(iface)),
            _ => false,
        }
    }

    /// Whether the interface type arguments `actual` implements fit
    /// those of `expected` (both found by `implements_interface`).
    fn iface_args_fit(&mut self, actual: &Ty, expected: &Ty) -> bool {
        let Ty::Adt(iface, want) = expected else {
            return true;
        };
        let (key, subst) = match actual {
            Ty::Adt(d, targs) => (impl_key_adt(*d), self.subst_of(*d, targs)),
            Ty::Prim(p) => (p.name().to_string(), HashMap::new()),
            _ => return true,
        };
        let Some(given) = self.impl_args.get(&(key, *iface)).cloned() else {
            return want.is_empty();
        };
        given.len() == want.len()
            && given.iter().zip(want).all(|(g, w)| {
                let g = self.inf.subst(g, &subst);
                self.inf.unify(&g, w)
            })
    }
}
