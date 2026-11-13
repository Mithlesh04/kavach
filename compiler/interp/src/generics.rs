//! The type arguments of a generic function's call. The interpreter runs
//! one body for every instance, so where the type matters at run time
//! (`json.decode<T>`, spec 2.12), it uses the arguments of the call that
//! is running: written at the call (`read<Point>(…)`), or found by
//! matching the function's written types with the checker's types at the
//! call. Native code makes one function per instance instead, so both
//! decode alike.

use crate::interp::Interp;
use kavach_ast::{Arg, Expr, ExprKind, FnDecl, ItemKind, TypeExpr, TypeKind};
use kavach_checker::Ty;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

impl<'a> Interp<'a> {
    /// `ty` with the running call's type arguments put in for its
    /// generic parameters.
    pub(crate) fn concrete(&self, ty: &Ty) -> Ty {
        let Some(generics) = self.frames.last().map(|f| &f.generics) else {
            return ty.clone();
        };
        if generics.is_empty() {
            return ty.clone();
        }
        subst(ty, generics)
    }

    /// The type arguments of the call `e` of generic function `f`.
    pub(crate) fn type_args(&self, f: &FnDecl, e: &Expr, args: &[Arg]) -> HashMap<DefId, Ty> {
        let mut out = HashMap::new();
        let types = &self.c.expr_types;
        if let ExprKind::Call { generics, .. } = &e.kind {
            for (g, written) in f.generics.iter().zip(generics) {
                let found = self.ty_of(written).map(|t| self.concrete(&t));
                if let (Some(&def), Some(t)) = (self.decl.get(&g.name.span.start), found) {
                    out.insert(def, t);
                }
            }
        }
        let result = types.get(&(e.span.start, e.span.end)).map(|t| match t {
            Ty::Result(ok, _) => (**ok).clone(),
            t => t.clone(),
        });
        if let (Some(written), Some(t)) = (&f.ret, result) {
            self.bind(written, &self.concrete(&t), &mut out);
        }
        // A method's `self` has no written type and no argument.
        for (p, a) in f.params.iter().filter(|p| p.ty.is_some()).zip(args) {
            let span = a.value.span;
            if let (Some(written), Some(t)) = (&p.ty, types.get(&(span.start, span.end))) {
                self.bind(written, &self.concrete(t), &mut out);
            }
        }
        out
    }

    /// Before calling method `f` at `call`: its type arguments, written or
    /// inferred, and those of the generic type it belongs to, from the
    /// receiver's type (`Box<Shout>`'s `I` is `Shout`), for the call to
    /// run with.
    pub(crate) fn method_generics(&mut self, f: &'a FnDecl, call: &'a Expr) {
        let ExprKind::Call { callee, args, .. } = &call.kind else {
            return;
        };
        let mut out = match f.generics.is_empty() {
            true => HashMap::new(),
            false => self.type_args(f, call, args),
        };
        if let ExprKind::Field { base, .. } = &callee.kind
            && let Some(t) = self.c.expr_types.get(&(base.span.start, base.span.end))
            && let Ty::Adt(def, ts) = self.concrete(t)
            && !ts.is_empty()
        {
            self.owner_generics(f, def, &ts, &mut out);
        }
        if !f.generics.is_empty() || !out.is_empty() {
            self.next_generics = Some(out);
        }
    }

    /// The parameters of generic type `def`, given `ts`, as method `f`
    /// names them: the type's own in its body, or an `extend<S> T<S>`
    /// block's (spec 2.5).
    fn owner_generics(&self, f: &FnDecl, def: DefId, ts: &[Ty], out: &mut HashMap<DefId, Ty>) {
        let has = |items: &[kavach_ast::Item]| {
            items
                .iter()
                .any(|m| matches!(&m.kind, ItemKind::Fn(g) if std::ptr::eq(g, f)))
        };
        if let Some(t) = self.records.get(&def)
            && let kavach_ast::TypeBody::Record(r) = &t.body
            && has(&r.methods)
        {
            for (g, ty) in t.generics.iter().zip(ts) {
                if let Some(&d) = self.decl.get(&g.name.span.start) {
                    out.entry(d).or_insert_with(|| ty.clone());
                }
            }
            return;
        }
        let extend = self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Extend(x) if has(&x.methods) => Some(x),
            _ => None,
        });
        if let Some(x) = extend {
            self.bind(&x.target, &Ty::Adt(def, ts.to_vec()), out);
        }
    }

    /// Matches written type `w` with `actual`, recording the generic
    /// parameters found.
    fn bind(&self, w: &TypeExpr, actual: &Ty, out: &mut HashMap<DefId, Ty>) {
        let both = |out: &mut HashMap<DefId, Ty>, ws: &[TypeExpr], ts: &[Ty]| {
            for (w, t) in ws.iter().zip(ts) {
                self.bind(w, t, out);
            }
        };
        match (&w.kind, actual) {
            (TypeKind::Named { path, args }, t) => {
                let res = &self.c.resolution;
                let Some(&def) = res.uses.get(&path.segments[0].span.start) else {
                    return;
                };
                if res.defs[def].kind == DefKind::GenericParam {
                    if !matches!(t, Ty::Param(_) | Ty::Error | Ty::Never) {
                        out.entry(def).or_insert_with(|| t.clone());
                    }
                    return;
                }
                match t {
                    Ty::Adt(_, ts) | Ty::Builtin(_, ts) => both(out, args, ts),
                    Ty::List(t) | Ty::Set(t) => both(out, args, std::slice::from_ref(t)),
                    Ty::Map(k, v) => both(out, args, &[(**k).clone(), (**v).clone()]),
                    _ => {}
                }
            }
            (TypeKind::List(w), Ty::List(t)) | (TypeKind::Optional(w), Ty::Optional(t)) => {
                self.bind(w, t, out)
            }
            (TypeKind::Map(wk, wv), Ty::Map(k, v)) => {
                self.bind(wk, k, out);
                self.bind(wv, v, out);
            }
            (TypeKind::Tuple(ws), Ty::Tuple(ts)) => both(out, ws, ts),
            // `wrap(show)`: a function type's parameters, result and error.
            (
                TypeKind::Fn {
                    params,
                    ret,
                    errors,
                    ..
                },
                Ty::Fn(ps, r, _),
            ) => {
                both(out, params, ps);
                let (ok, err) = match &**r {
                    Ty::Result(ok, err) => (&**ok, Some(&**err)),
                    r => (r, None),
                };
                if let Some(w) = ret {
                    self.bind(w, ok, out);
                }
                if let (Some(w), Some(e)) = (errors, err) {
                    self.bind(w, e, out);
                }
            }
            _ => {}
        }
    }
}

/// `ty` with `generics` put in for its generic parameters.
fn subst(ty: &Ty, generics: &HashMap<DefId, Ty>) -> Ty {
    let all = |ts: &[Ty]| ts.iter().map(|t| subst(t, generics)).collect();
    let one = |t: &Ty| Box::new(subst(t, generics));
    match ty {
        Ty::Param(d) => generics.get(d).cloned().unwrap_or(Ty::Param(*d)),
        Ty::List(t) => Ty::List(one(t)),
        Ty::Set(t) => Ty::Set(one(t)),
        Ty::Optional(t) => Ty::Optional(one(t)),
        Ty::Range(t) => Ty::Range(one(t)),
        Ty::Map(k, v) => Ty::Map(one(k), one(v)),
        Ty::Tuple(ts) => Ty::Tuple(all(ts)),
        Ty::Fn(ps, r, u) => Ty::Fn(all(ps), one(r), u.clone()),
        Ty::Adt(d, ts) => Ty::Adt(*d, all(ts)),
        Ty::Builtin(n, ts) => Ty::Builtin(n, all(ts)),
        Ty::Result(ok, err) => Ty::Result(one(ok), one(err)),
        t => t.clone(),
    }
}
