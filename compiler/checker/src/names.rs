//! The type of a name used as a value: a local, a function, a type
//! through which a variant or constructor is reached, a field inside an
//! `invariant`, and the tags of literals, which can't stand alone.

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefKind;

impl Checker<'_> {
    pub(crate) fn name(&mut self, id: &Ident, expected: Option<&Ty>) -> Ty {
        if let Some(candidates) = self.res.ambiguous.get(&id.span.start).cloned() {
            let want = expected.map(|t| self.inf.shallow(t));
            for c in &candidates {
                if let DefKind::Variant { of } = self.res.defs[*c].kind
                    && matches!(&want, Some(Ty::Adt(d, _)) if *d == of)
                {
                    self.bare_variant(id, of);
                    return self.instance(of).0;
                }
            }
            let unions: Vec<String> = (self.imported_variants(&candidates).iter())
                .filter_map(|c| match self.res.defs[*c].kind {
                    DefKind::Variant { of } => Some(format!("`{}.{}`", self.def_name(of), id.name)),
                    _ => None,
                })
                .collect();
            self.error(
                id.span,
                format!(
                    "`{}` is a variant of several types: write {}",
                    id.name,
                    unions.join(" or ")
                ),
            );
            return Ty::Error;
        }
        let Some(&def) = self.res.uses.get(&id.span.start) else {
            return Ty::Error;
        };
        match self.res.defs[def].kind {
            DefKind::Local { .. } | DefKind::Param => {
                self.locals.get(&def).cloned().unwrap_or(Ty::Error)
            }
            DefKind::Result => self.ctx.as_ref().map_or(Ty::Error, |c| c.ret.clone()),
            DefKind::Field => self.record_field(&id.name).unwrap_or(Ty::Error),
            DefKind::Fn => match self.fns.get(&def).cloned() {
                Some(sig) if sig.params.iter().any(|p| p.inout) => {
                    self.error(
                        id.span,
                        format!("`{}` has `inout` parameters, so it can't be a function value (spec 3.9)", id.name),
                    );
                    Ty::Error
                }
                Some(sig) => {
                    // A function used as a value may be called later.
                    self.note_fn(def, id.span);
                    let subst = sig
                        .generics
                        .iter()
                        .map(|(g, _)| (*g, self.fresh()))
                        .collect();
                    let params = sig
                        .params
                        .iter()
                        .map(|p| self.inf.subst(&p.ty, &subst))
                        .collect();
                    let ret = self.inf.subst(&sig.ret, &subst);
                    // Its errors are part of its type (spec 3.4).
                    let ret = match &sig.errors {
                        Some(err) => {
                            Ty::Result(Box::new(ret), Box::new(self.inf.subst(err, &subst)))
                        }
                        None => ret,
                    };
                    Ty::Fn(params, Box::new(ret), None)
                }
                None => Ty::Error,
            },
            DefKind::Variant { of } => {
                let has_fields = self.variant_fields(of, def).is_some_and(|f| !f.is_empty());
                if has_fields {
                    let union = self.def_name(of).to_string();
                    self.error(
                        id.span,
                        format!(
                            "`{}` has fields: write `{union}.{}(…)` with them",
                            id.name, id.name
                        ),
                    );
                } else {
                    self.bare_variant(id, of);
                }
                self.instance(of).0
            }
            DefKind::Type | DefKind::Interface | DefKind::Capability | DefKind::Agent => {
                self.error(
                    id.span,
                    format!(
                        "`{}` is a type, not a value: create one with `{}(…)`",
                        id.name, id.name
                    ),
                );
                Ty::Error
            }
            DefKind::Module => {
                self.error(
                    id.span,
                    format!(
                        "`{0}` is a module, not a value: use one of its names, like `{0}.name`",
                        id.name
                    ),
                );
                Ty::Error
            }
            // `count` in a `limit` rule (spec 8.3).
            DefKind::Builtin if matches!(self.res.defs[def].name.as_str(), "count" | "tokens") => {
                Ty::Prim(Prim::Int)
            }
            DefKind::Builtin | DefKind::Import | DefKind::GenericParam => Ty::Error,
            // A tag goes right before its text (spec 10.15).
            DefKind::Literal => {
                self.error(id.span, format!("`{0}` is a tag for literals: write the text right after it, `{0}\"…\"` (spec 10.15)", id.name));
                Ty::Error
            }
        }
    }

    fn record_field(&self, name: &str) -> Option<Ty> {
        match &self.types.get(&self.record?)?.shape {
            Shape::Record(fields) => fields.iter().find(|f| f.name == name).map(|f| f.ty.clone()),
            _ => None,
        }
    }
}
