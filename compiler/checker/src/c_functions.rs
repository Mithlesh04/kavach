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
        // A record of those goes by value, as a C `struct` does.
        let record = |c: &Self, t: &Ty| matches!(t, Ty::Adt(..)) && c.c_pointee(t);
        for (p, info) in f.params.iter().zip(&sig.params) {
            let text = matches!(info.ty, Ty::Prim(Prim::String | Prim::Bytes));
            let fits = plain(self, &info.ty) || record(self, &info.ty);
            if !fits && !text && !callback(self, &info.ty) {
                let shown = self.show(&info.ty);
                self.error(
                    p.name.span,
                    format!("a C function takes numbers, booleans, `Ptr`s, records of those, `String`s, `Bytes` and callbacks (for now), not `{shown}` (spec 10.7)"),
                );
            }
        }
        if !plain(self, &sig.ret) && !record(self, &sig.ret) {
            let shown = self.show(&sig.ret);
            self.error(
                f.name.span,
                format!("a C function gives a number, a boolean, a `Ptr`, a record of those or nothing (for now), not `{shown}` (spec 10.7)"),
            );
        }
        if f.errors.is_some() || !f.uses.is_empty() || !f.generics.is_empty() {
            self.error(
                f.name.span,
                "a C function has no `!`, `uses` or type parameters: C has none of them (spec 10.7)",
            );
        }
    }

    /// A call of a C function, or of `open` or a function of a library
    /// loaded while running, is made inside `unsafe` (spec 10.7).
    pub(crate) fn c_needs_unsafe(&mut self, sig: &FnSig, span: kavach_ast::Span, name: &str) {
        if self.in_unsafe == 0 && sig.def.is_some_and(|d| self.c_fns.contains_key(&d)) {
            self.error(
                span,
                format!("`{name}` is a C function: call it inside `unsafe {{ … }}`, in a safe function around it (spec 10.7)"),
            );
        }
    }

    /// A function of `extern c library` (spec 10.7): its C types are
    /// checked as any C function's, and it is called inside `unsafe`.
    pub(crate) fn note_loaded(&mut self, f: &kavach_ast::FnDecl, sig: &FnSig) {
        let Some(loaded) = f.c_loaded else {
            return;
        };
        if loaded != kavach_ast::CLoaded::Open {
            // Its `self` is the library, not one of C's parameters.
            let mut own = f.clone();
            own.params.retain(|p| p.name.name != "self");
            self.check_c_signature(&own, sig);
        }
        if let Some(&def) = self.decl.get(&f.name.span.start) {
            self.c_fns.insert(def, None);
        }
    }

    /// `...` is C's alone: a Kavach function takes a list (spec 3.1), and
    /// C needs a parameter before it (spec 10.7).
    pub(crate) fn only_c_takes_more(&mut self, f: &kavach_ast::FnDecl) {
        if f.c_variadic && f.c_library.is_none() {
            self.error(
                f.name.span,
                "only a C function takes `...`: a Kavach function takes more values in a list (spec 3.1)",
            );
        } else if f.c_variadic && f.params.is_empty() {
            self.error(
                f.name.span,
                "C needs a parameter before `...`, as in `fn printf(format: String, ...)` (spec 10.7)",
            );
        }
    }

    /// A C function's arguments checked as C takes them: callbacks, and
    /// those after its own parameters (`...`), each a number, a `Ptr` or a
    /// `String` (spec 10.7). The arguments for its own
    /// parameters, to check against its signature.
    pub(crate) fn c_arguments<'x>(&mut self, c_fn: DefId, args: &'x [Arg]) -> &'x [Arg] {
        self.check_callbacks(c_fn, args);
        let Some(&Some(own)) = self.c_fns.get(&c_fn) else {
            return args;
        };
        for a in args.iter().skip(own) {
            let t = self.value(&a.value, None);
            let t = self.inf.shallow(&t);
            let fits = match &t {
                // A number literal: an `Int` or a `Float` once settled.
                Ty::Var(v) => self.inf.kind(*v) != crate::infer::VarKind::Any,
                Ty::Builtin("Ptr", _) => true,
                Ty::Prim(Prim::String) => true,
                t => number(t) && !matches!(t, Ty::Prim(Prim::Decimal | Prim::BigInt | Prim::Bool)),
            };
            if !fits && t != Ty::Error {
                let shown = self.show(&t);
                self.error(
                    a.value.span,
                    format!(
                        "C's `...` takes numbers, `Ptr`s and `String`s, not `{shown}` (spec 10.7)"
                    ),
                );
            }
        }
        &args[..own.min(args.len())]
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
                let plain = self.res.defs[d].kind == DefKind::Fn && !self.c_fns.contains_key(&d);
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
    /// Each `Ptr<T>` written names a type C lays out (spec 6.4).
    pub(crate) fn check_pointees(&mut self) {
        let mut told = std::collections::HashSet::new();
        for (t, span) in std::mem::take(&mut self.ptr_checks) {
            let known = !matches!(t, Ty::Param(_) | Ty::Var(_) | Ty::Error);
            if known && !self.c_pointee(&t) && told.insert(span.start) {
                let shown = self.show(&t);
                self.error(
                    span,
                    format!("a `Ptr` points at memory C lays out: numbers, `Bool`, `Ptr`s or a record of those, not `{shown}` (spec 6.4)"),
                );
            }
        }
    }

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
