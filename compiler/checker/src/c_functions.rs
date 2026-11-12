//! C functions (`extern c`, spec 10.7): the types their signatures may
//! use, and the Kavach functions C may call back.

use crate::checker::{Checker, FnSig, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, ExprKind};
use kavach_resolver::{DefId, DefKind};

impl Checker<'_> {
    /// A C function (spec 10.7) takes and gives what C passes the same
    /// way (numbers, booleans, pointers; text and callbacks going in), and
    /// has no clauses Kavach can't keep.
    pub(crate) fn check_c_signature(&mut self, f: &kavach_ast::FnDecl, sig: &FnSig) {
        let plain = |c: &Self, t: &Ty| match t {
            Ty::Unit => true,
            Ty::Builtin("Ptr", args) => args.first().is_some_and(|p| c.c_pointee(p)),
            t => number(t),
        };
        // A callback: a C function pointer, with C types only.
        let callback = |c: &Self, t: &Ty| match t {
            Ty::Fn(params, ret, _) => {
                params.iter().all(|p| *p != Ty::Unit && plain(c, p)) && plain(c, ret)
            }
            _ => false,
        };
        for (p, info) in f.params.iter().zip(&sig.params) {
            let text = info.ty == Ty::Prim(Prim::String);
            if !plain(self, &info.ty) && !text && !callback(self, &info.ty) {
                let shown = self.show(&info.ty);
                self.error(
                    p.name.span,
                    format!("a C function takes numbers, booleans, `Ptr`s, `String`s and callbacks of those (for now), not `{shown}` (spec 10.7)"),
                );
            }
        }
        if !plain(self, &sig.ret) {
            let shown = self.show(&sig.ret);
            self.error(
                f.name.span,
                format!("a C function gives a number, a boolean, a `Ptr` or nothing (for now), not `{shown}` (spec 10.7)"),
            );
        }
        if f.errors.is_some() || !f.uses.is_empty() || !f.generics.is_empty() {
            self.error(
                f.name.span,
                "a C function has no `!`, `uses` or type parameters: C has none of them (spec 10.7)",
            );
        }
    }

    /// Each argument C calls back (spec 10.7) is a named Kavach function
    /// that can't fail: C can't keep what a closure captures, nor take an
    /// error back.
    pub(crate) fn check_callbacks(&mut self, c_fn: DefId, args: &[Arg]) {
        let Some(sig) = self.fns.get(&c_fn).cloned() else {
            return;
        };
        for (param, arg) in sig.params.iter().zip(args) {
            if !matches!(param.ty, Ty::Fn(..)) {
                continue;
            }
            let named = match &arg.value.kind {
                ExprKind::Name(id) => self.res.uses.get(&id.span.start).copied(),
                _ => None,
            };
            let ok = named.and_then(|d| {
                let f = self.fns.get(&d)?;
                let plain = self.res.defs[d].kind == DefKind::Fn && !self.c_fns.contains(&d);
                Some(plain && f.errors.is_none() && f.generics.is_empty())
            });
            if ok != Some(true) {
                self.error(
                    arg.value.span,
                    "C calls back only a named Kavach function that can't fail (no `!`): C has nowhere to keep what a lambda or closure captures (spec 10.7)",
                );
            }
        }
    }

    /// What a `Ptr` may point to (spec 6.4): a number, a `Bool`, a `Ptr`,
    /// or a record of only those, laid out as a C struct.
    fn c_pointee(&self, t: &Ty) -> bool {
        match t {
            Ty::Builtin("Ptr", _) => true,
            Ty::Adt(def, args) if args.is_empty() => match self.types.get(def) {
                Some(info) => match &info.shape {
                    Shape::Record(fields) => fields
                        .iter()
                        .all(|f| number(&f.ty) || matches!(f.ty, Ty::Builtin("Ptr", _))),
                    _ => false,
                },
                // Declared further down: the native compiler checks it.
                None => true,
            },
            t => number(t),
        }
    }
}

/// A number or a `Bool`: what C passes as it is.
fn number(t: &Ty) -> bool {
    matches!(t, Ty::Prim(p) if p.is_numeric() && *p != Prim::Decimal || *p == Prim::Bool)
}
