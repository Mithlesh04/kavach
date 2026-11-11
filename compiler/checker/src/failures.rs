//! Errors as values (spec 5): `try`, `fail`, `return`, and which error
//! types fit together; plus the `unsafe` opt-in (spec 6.4).

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;

const STRING: Ty = Ty::Prim(Prim::String);

impl Checker<'_> {
    pub(crate) fn fail(&mut self, e: &Expr, x: &Expr) -> Ty {
        match self
            .ctx
            .as_ref()
            .map(|c| (c.name.clone(), c.errors.clone()))
        {
            Some((_, Some(err))) => {
                if matches!(err, Ty::Builtin("Error", _)) {
                    self.value(x, None);
                } else {
                    self.check_expect(x, &err);
                }
            }
            Some((name, None)) => {
                self.value(x, None);
                // In a `gpu fn`, the GPU's own message says why (spec 15.1).
                if !self.in_gpu_fn() {
                    self.error(e.span, format!("`fail` returns an error, but `{name}` has no `!` in its signature: add `! ErrorType`"));
                }
            }
            None => {}
        }
        Ty::Never
    }

    pub(crate) fn return_expr(&mut self, e: &Expr, value: Option<&Expr>) -> Ty {
        let ret = self.ctx.as_ref().map_or(Ty::Error, |c| c.ret.clone());
        match value {
            Some(v) => {
                self.check_expect(v, &ret);
            }
            None if !matches!(ret, Ty::Unit | Ty::Error) => {
                let shown = self.show(&ret);
                self.error(
                    e.span,
                    format!("`return` needs a value of type `{shown}` here"),
                );
            }
            None => {}
        }
        Ty::Never
    }

    pub(crate) fn try_expr(&mut self, expr: &Expr, context: Option<&Expr>) -> Ty {
        if let Some(c) = context {
            self.check_expect(c, &STRING);
        }
        let (ok, err) = match self.infer(expr, None) {
            Ty::Result(ok, err) => (ok, err),
            Ty::Error => return Ty::Error,
            t => {
                self.warn(expr.span, "`try` does nothing here: this can't fail");
                return t;
            }
        };
        let Some((name, own)) = self
            .ctx
            .as_ref()
            .map(|c| (c.name.clone(), c.errors.clone()))
        else {
            return *ok;
        };
        match own {
            None if self.in_gpu_fn() => {}
            None => self.error(expr.span, format!(
                "`try` passes the error on, but `{name}` has no `!` in its signature: add `! {}`, or handle the error with `catch`",
                self.show(&err)
            )),
            Some(own) if !self.error_fits(&err, &own) => {
                let (e, o) = (self.show(&err), self.show(&own));
                self.error(expr.span, format!("`try` can't pass on `{e}`: `{name}` only returns `! {o}`"));
            }
            Some(_) => {}
        }
        *ok
    }

    /// Whether error type `err` is part of `own` (spec 5.2): the same type,
    /// the `Error` catch-all, or a union with a variant of that name.
    /// Whether the function being checked is a `gpu fn`, which can't fail
    /// at all: its own message says so (`gpu_walk.rs`).
    pub(crate) fn in_gpu_fn(&self) -> bool {
        self.current_fn.is_some_and(|f| self.gpu_fns.contains(&f))
    }

    pub(crate) fn error_fits(&mut self, err: &Ty, own: &Ty) -> bool {
        if matches!(own, Ty::Builtin("Error", _)) || self.inf.unify(err, own) {
            return true;
        }
        let (Ty::Adt(own_def, _), Ty::Adt(err_def, _)) =
            (self.inf.resolve(own), self.inf.resolve(err))
        else {
            return false;
        };
        let err_name = self.def_name(err_def).to_string();
        matches!(&self.types.get(&own_def).map(|t| &t.shape), Some(Shape::Union(vs)) if vs.iter().any(|v| v.name == err_name))
    }

    pub(crate) fn check_unsafe(&mut self, span: Span) {
        let package = &self.res.modules[self.res.module_at(span.start)].package;
        let allowed = match package.is_empty() {
            true => self.opts.allow_unsafe,
            false => self.opts.unsafe_packages.contains(package),
        };
        if !allowed {
            self.error(span, "this package doesn't allow `unsafe`: set `unsafe = true` in kavach.toml to use it (spec 6.4)");
        }
    }
}

impl Checker<'_> {
    /// A warning at `span`, unless it is in the standard library.
    pub(crate) fn warn(&mut self, span: Span, message: &str) {
        // Warnings in a dependency are its authors' to fix (spec 10.12).
        let module = &self.res.modules[self.res.module_at(span.start)];
        if module.name.starts_with("std.") || !module.package.is_empty() {
            return;
        }
        self.warnings.push(crate::Diagnostic {
            message: message.to_string(),
            span,
        });
    }
}

impl Checker<'_> {
    /// `call catch handler` or `call catch e => handler` (spec 5.3).
    pub(crate) fn catch_expr(
        &mut self,
        expr: &Expr,
        binding: Option<&Ident>,
        handler: &Expr,
        expected: Option<&Ty>,
    ) -> Ty {
        match self.infer(expr, expected) {
            Ty::Result(ok, err) => {
                if let Some(def) = binding.and_then(|b| self.decl.get(&b.span.start).copied()) {
                    self.locals.insert(def, *err);
                }
                self.check_expect(handler, &ok);
                *ok
            }
            t => {
                if t != Ty::Error {
                    self.warn(expr.span, "`catch` does nothing here: this can't fail");
                }
                self.value(handler, None);
                t
            }
        }
    }
}
