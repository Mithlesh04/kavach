//! Function values that can fail (spec 3.4, 5.2, decision 0085): a
//! function's errors are part of its type, and a built-in method given a
//! function that can fail passes its error on.

use crate::capabilities::Need;
use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::{Expr, ExprKind, Span};
use kavach_resolver::{DefId, DefKind};
use std::collections::{BTreeSet, HashMap};

/// A function value as it was made, for checking a `uses` limit once
/// capabilities are known.
#[derive(Clone, Debug)]
pub(crate) enum Made {
    /// A named function, where it is used as a value.
    Named(DefId, String, Span),
    /// A lambda, by its body's place.
    Lambda(Span),
    /// Anything else: a variable, a call, a field.
    Other(Span),
}

impl Checker<'_> {
    /// Whether function type `actual` fits where `expected` is needed: one
    /// that can't fail fits one that can, never the other way round.
    pub(crate) fn fn_fits(&mut self, actual: &Ty, expected: &Ty) -> bool {
        let (Ty::Fn(ap, ar, _), Ty::Fn(ep, er, _)) = (actual, expected) else {
            return false;
        };
        if ap.len() != ep.len() || !ap.iter().zip(ep).all(|(a, e)| self.inf.unify(a, e)) {
            return false;
        }
        if let Ty::Fn(.., Some(limit)) = expected
            && !self.fits_limit(actual, limit)
        {
            return false;
        }
        match (self.inf.shallow(ar), self.inf.shallow(er)) {
            (Ty::Result(ok, err), Ty::Result(want_ok, want_err)) => {
                self.inf.unify(&ok, &want_ok) && self.error_fits(&err, &want_err)
            }
            (Ty::Result(ok, err), want) => match self.rethrow.as_mut() {
                // A built-in method passes the error on.
                Some(errors) => {
                    errors.push(*err);
                    self.inf.unify(&ok, &want)
                }
                None => false,
            },
            (r, Ty::Result(want_ok, _)) => self.inf.unify(&r, &want_ok),
            (r, want) => self.inf.unify(&r, &want),
        }
    }

    /// Checks a built-in method's arguments with `check`, and gives its
    /// result `ret`, failing with the error of a function given to it.
    pub(crate) fn passing_errors_on(
        &mut self,
        ret: Ty,
        span: kavach_ast::Span,
        check: impl FnOnce(&mut Self),
    ) -> Ty {
        let outer = self.rethrow.replace(Vec::new());
        check(self);
        let errors = std::mem::replace(&mut self.rethrow, outer).unwrap_or_default();
        let Some(first) = errors.first().cloned() else {
            return ret;
        };
        for e in &errors[1..] {
            if !self.inf.unify(e, &first) {
                let (a, b) = (self.show(&first), self.show(e));
                self.error(span, format!("the functions given here fail with `{a}` and `{b}`: give them one error type (spec 5.2)"));
            }
        }
        Ty::Result(Box::new(ret), Box::new(first))
    }

    /// How `e` makes a function value.
    pub(crate) fn made(&self, e: &Expr) -> Made {
        match &e.kind {
            ExprKind::Lambda { body, .. } | ExprKind::DotLambda(body) => Made::Lambda(body.span),
            ExprKind::Name(id) => match self.res.uses.get(&id.span.start) {
                Some(&d) if matches!(self.res.defs[d].kind, DefKind::Fn) => {
                    Made::Named(d, id.name.clone(), id.span)
                }
                _ => Made::Other(e.span),
            },
            _ => Made::Other(e.span),
        }
    }

    /// Whether a value of function type `actual` keeps to `limit`: its
    /// own limit within it, or, for a function or lambda made here, what
    /// it uses, checked once that is known (`limits_broken`).
    fn fits_limit(&mut self, actual: &Ty, limit: &[String]) -> bool {
        if let Ty::Fn(.., Some(own)) = actual {
            return own.iter().all(|c| covers(limit, c));
        }
        if let Some(made) = self.fitting.clone() {
            self.uses_limits.push((made, limit.to_vec()));
        }
        true
    }

    /// The errors of `uses` limits broken, with what each function uses
    /// (`effective`) and each lambda needs.
    pub(crate) fn limits_broken(
        &mut self,
        effective: &HashMap<DefId, BTreeSet<String>>,
        lambdas: &HashMap<(u32, u32), Vec<Need>>,
    ) -> Vec<(Span, String)> {
        let mut errors = Vec::new();
        for (made, limit) in std::mem::take(&mut self.uses_limits) {
            let shown = limit.join(", ");
            let (what, used, at) = match &made {
                Made::Named(d, name, at) => (
                    format!("`{name}`"),
                    effective.get(d).cloned().unwrap_or_default(),
                    *at,
                ),
                Made::Lambda(body) => {
                    let mut used = BTreeSet::new();
                    for n in lambdas.get(&(body.start, body.end)).into_iter().flatten() {
                        match n {
                            Need::Cap(c) => {
                                used.insert(c.clone());
                            }
                            Need::Fn(f) => {
                                used.extend(effective.get(f).cloned().unwrap_or_default())
                            }
                        }
                    }
                    ("this lambda".to_string(), used, *body)
                }
                // Its type sets no limit, so it may use anything.
                Made::Other(at) => {
                    errors.push((*at, format!("this function value's type sets no `uses` limit, so it can't be given where only {shown} is allowed: write `uses` in its type (spec 3.4)")));
                    continue;
                }
            };
            for cap in used.iter().filter(|c| !covers(&limit, c)) {
                errors.push((at, format!("{what} uses {cap}, but the function type it is given as allows only {shown} (spec 3.4)")));
            }
        }
        errors
    }

    /// `json.describe(f)` (spec 2.12): `f` must name a function, which is
    /// described, not used, so it costs no capability here. Its whole
    /// type is noted at the argument, for the description.
    pub(crate) fn describe_call(&mut self, args: &[kavach_ast::Arg], span: Span) -> Ty {
        let fn_schema = self.builtin_ty("FnSchema");
        let named = match args {
            [a] if a.name.is_none() => match &a.value.kind {
                ExprKind::Name(id) => self
                    .res
                    .uses
                    .get(&id.span.start)
                    .copied()
                    .filter(|d| matches!(self.res.defs[*d].kind, DefKind::Fn))
                    .map(|d| (d, a.value.span)),
                _ => None,
            },
            _ => None,
        };
        let Some((def, at)) = named else {
            // A function value: described when it is a named function
            // (decision D1 in spec 2.12), `none` for a lambda.
            if let [a] = args
                && a.name.is_none()
            {
                let t = self.value(&a.value, None);
                if matches!(self.inf.resolve(&t), Ty::Fn(..)) {
                    return Ty::Optional(Box::new(fn_schema));
                }
            }
            self.error(
                span,
                "`json.describe` takes a function: `json.describe(create)` (spec 2.12)",
            );
            return fn_schema;
        };
        let Some(sig) = self.fns.get(&def).cloned() else {
            return fn_schema;
        };
        if !sig.generics.is_empty() {
            self.error(at, "a generic function has no one description: describe a function that calls it with the types it takes (spec 2.12)");
            return fn_schema;
        }
        let mut parts: Vec<Ty> = sig.params.iter().map(|p| p.ty.clone()).collect();
        parts.push(sig.ret.clone());
        parts.extend(sig.errors.clone());
        for t in &parts {
            let t = self.inf.resolve(t);
            if matches!(t, Ty::Fn(..) | Ty::Tuple(_))
                || matches!(&t, Ty::Map(k, _) if **k != Ty::Prim(crate::Prim::String))
            {
                let shown = self.show(&t);
                self.error(at, format!("`{shown}` has no JSON Schema, so this function can't be described (spec 2.12)"));
            }
        }
        let t = self.fn_value_ty(&sig);
        self.expr_types.push((at, t));
        fn_schema
    }

    /// A function's type as a value: its parameters, and its result with
    /// its error.
    pub(crate) fn fn_value_ty(&self, sig: &crate::checker::FnSig) -> Ty {
        let ret = match &sig.errors {
            Some(e) => Ty::Result(Box::new(sig.ret.clone()), Box::new(e.clone())),
            None => sig.ret.clone(),
        };
        let params = sig.params.iter().map(|p| p.ty.clone()).collect();
        Ty::Fn(params, Box::new(ret), None)
    }
}

/// Whether `limit` covers `cap`: `Fs` covers `Fs.read`.
fn covers(limit: &[String], cap: &str) -> bool {
    limit
        .iter()
        .any(|l| l == cap || cap.split_once('.').is_some_and(|(c, _)| c == l))
}
