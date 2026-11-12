//! A type that says it implements an interface keeps the promise (spec
//! 2.8): it has each of the interface's methods, taking as many values,
//! with the same `inout` parameters (3.9), failing exactly when the
//! interface's method can fail (chapter 5), and with the same types once
//! `Self` and the interface's type arguments are put in.

use crate::checker::{Checker, FnSig};
use crate::ty::Ty;
use kavach_ast::*;
use kavach_resolver::DefKind;
use std::collections::HashMap;

impl Checker<'_> {
    pub(crate) fn check_conformance(&mut self, file: &File) {
        for item in &file.items {
            match &item.kind {
                ItemKind::Type(t) => {
                    let own = self.decl.get(&t.name.span.start).copied();
                    let methods = own.and_then(|d| self.methods.get(&d)).cloned();
                    let self_ty = own.map(|d| {
                        let generics = self.types.get(&d).map(|t| t.generics.clone());
                        let args = generics.unwrap_or_default().into_iter().map(Ty::Param);
                        Ty::Adt(d, args.collect())
                    });
                    self.conforms(&t.name.name, self_ty, methods, &t.implements);
                }
                ItemKind::Extend(e) if !e.implements.is_empty() => {
                    let TypeKind::Named { path, .. } = &e.target.kind else {
                        continue;
                    };
                    let Some(last) = path.segments.last() else {
                        continue;
                    };
                    let methods =
                        match (crate::ty::Prim::from_name(&last.name), self.def_named(last)) {
                            (Some(p), _) => self.ext_methods.get(p.name()).cloned(),
                            (None, Some(d)) => self.methods.get(&d).cloned(),
                            (None, None) => None,
                        };
                    let self_ty = self.lower(&e.target, None);
                    self.conforms(&last.name, Some(self_ty), methods, &e.implements);
                }
                _ => {}
            }
        }
    }

    /// The definition `id` names, if it names one.
    fn def_named(&self, id: &Ident) -> Option<kavach_resolver::DefId> {
        self.res.uses.get(&id.span.start).copied()
    }

    /// Checks type `name`, which is `self_ty` and has `methods`, against
    /// each interface in `implements`.
    fn conforms(
        &mut self,
        name: &str,
        self_ty: Option<Ty>,
        methods: Option<HashMap<String, FnSig>>,
        implements: &[TypeExpr],
    ) {
        let methods = methods.unwrap_or_default();
        for i in implements {
            let TypeKind::Named { path, .. } = &i.kind else {
                continue;
            };
            let Some(iface) = path.segments.last().and_then(|id| self.def_named(id)) else {
                continue;
            };
            if self.res.defs[iface].kind == DefKind::Type {
                // `type Dog: Animal` where `Animal` is a record: no
                // inheritance (decision 0029).
                let parent = self.res.defs[iface].name.clone();
                let field = parent.to_lowercase();
                self.error(
                    i.span,
                    format!("`{name}` can't extend `{parent}`: `{parent}` is a type, not an interface, and Kavach has no inheritance (decision 0029). Give `{name}` a field `{field}: {parent}`, or make `{parent}` an interface"),
                );
                continue;
            }
            if self.res.defs[iface].kind != DefKind::Interface {
                continue;
            }
            let iface_name = self.res.defs[iface].name.clone();
            // What the interface's types mean for this type: `Self` is
            // `self_ty`, and its generics are the written arguments.
            let shown = self.lower(i, self_ty.as_ref());
            let mut subst = match &shown {
                Ty::Adt(_, args) => self.subst_of(iface, args),
                _ => HashMap::new(),
            };
            subst.insert(iface, self_ty.clone().unwrap_or(Ty::Error));
            let iface_shown = self.show(&shown);
            let mut wanted: Vec<(String, FnSig)> = self
                .methods
                .get(&iface)
                .map(|m| m.iter().map(|(n, s)| (n.clone(), s.clone())).collect())
                .unwrap_or_default();
            wanted.sort_by(|a, b| a.0.cmp(&b.0));
            for (method, want) in wanted {
                let what = format!("`{name}.{method}`");
                let Some(have) = methods.get(&method) else {
                    self.error(
                        i.span,
                        format!("`{name}` says it implements `{iface_name}`, so it needs the method `{method}`"),
                    );
                    continue;
                };
                if have.params.len() != want.params.len() {
                    self.error(
                        i.span,
                        format!(
                            "{what} takes {} values, and `{iface_name}.{method}` takes {}: they must match",
                            have.params.len(),
                            want.params.len()
                        ),
                    );
                    continue;
                }
                for (h, w) in have.params.iter().zip(&want.params) {
                    if h.inout != w.inout {
                        let (is, not) = if w.inout { ("", "not ") } else { ("not ", "") };
                        self.error(
                            i.span,
                            format!(
                                "`{}` of {what} is {not}`inout`, and in `{iface_name}.{method}` it is {is}`inout`: they must match",
                                h.name
                            ),
                        );
                    }
                }
                let theirs = format!("`{iface_shown}.{method}`");
                self.same_types(i.span, &what, &theirs, have, &want, subst.clone());
                if have.errors.is_some() != want.errors.is_some() {
                    let (can, cant) = if want.errors.is_some() {
                        ("can't", "can")
                    } else {
                        ("can", "can't")
                    };
                    self.error(
                        i.span,
                        format!(
                            "{what} {can} fail, and `{iface_name}.{method}` {cant}: they must match"
                        ),
                    );
                }
            }
        }
    }

    /// Reports each type of `have` that isn't the type of `want`, read
    /// with `subst`. A method's own generics are matched by position.
    fn same_types(
        &mut self,
        span: kavach_ast::Span,
        what: &str,
        theirs: &str,
        have: &FnSig,
        want: &FnSig,
        mut subst: HashMap<kavach_resolver::DefId, Ty>,
    ) {
        if have.generics.len() != want.generics.len() {
            self.error(
                span,
                format!(
                    "{what} has {} type parameters, and {theirs} has {}: they must match",
                    have.generics.len(),
                    want.generics.len()
                ),
            );
            return;
        }
        // On the method itself, so each wrong method gets its own message.
        let span = (have.def)
            .and_then(|d| self.res.defs[d].span)
            .unwrap_or(span);
        let mut own = HashMap::new();
        for ((h, _), (w, _)) in have.generics.iter().zip(&want.generics) {
            own.insert(*h, Ty::Param(*w));
            subst.entry(*w).or_insert(Ty::Param(*w));
        }
        // Both say `-> secret` or neither (spec 8.7).
        if have.ret_secret != want.ret_secret {
            let (yes, no) = match have.ret_secret {
                true => (what, theirs),
                false => (theirs, what),
            };
            self.error(
                span,
                format!("{yes} returns a secret (`-> secret …`) and {no} doesn't: they must agree (spec 8.7)"),
            );
        }
        let mut pairs: Vec<(String, String, &Ty, &Ty)> = Vec::new();
        for (h, w) in have.params.iter().zip(&want.params) {
            let mine = format!("`{}` of {what} is", h.name);
            pairs.push((mine, format!("in {theirs} it is"), &h.ty, &w.ty));
        }
        let gives = (format!("{what} gives"), format!("{theirs} gives"));
        pairs.push((gives.0, gives.1, &have.ret, &want.ret));
        if let (Some(h), Some(w)) = (&have.errors, &want.errors) {
            pairs.push((format!("{what} fails with"), format!("{theirs} with"), h, w));
        }
        for (mine, them, h, w) in pairs {
            let (h, w) = (self.inf.subst(h, &own), self.inf.subst(w, &subst));
            let (h, w) = (self.inf.resolve(&h), self.inf.resolve(&w));
            if h == w || unknown(&h) || unknown(&w) {
                continue;
            }
            let (hs, ws) = (self.show(&h), self.show(&w));
            self.error(
                span,
                format!("{mine} `{hs}`, and {them} `{ws}`: they must match"),
            );
        }
    }
}

/// Whether `ty` holds a type the checker couldn't work out, so comparing it
/// would only repeat an earlier error.
fn unknown(ty: &Ty) -> bool {
    match ty {
        Ty::Error | Ty::Var(_) => true,
        Ty::List(t) | Ty::Set(t) | Ty::Optional(t) | Ty::Range(t) => unknown(t),
        Ty::Map(a, b) | Ty::Result(a, b) => unknown(a) || unknown(b),
        Ty::Tuple(ts) | Ty::Adt(_, ts) | Ty::Builtin(_, ts) => ts.iter().any(unknown),
        Ty::Anon(fs) => fs.iter().any(|(_, t)| unknown(t)),
        Ty::Fn(ps, r, _) => ps.iter().any(unknown) || unknown(r),
        _ => false,
    }
}
