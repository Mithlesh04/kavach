//! A generic type's instances: fresh type arguments for each use, or the
//! ones written at a construction (`Box<Shout>(raw: …)`, spec 2.9).

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_resolver::DefId;
use std::collections::HashMap;

impl Checker<'_> {
    /// The type of a fresh instance of a record or union: `Pair<?, ?>`.
    pub(crate) fn instance(&mut self, def: DefId) -> (Ty, HashMap<DefId, Ty>) {
        let generics = self
            .types
            .get(&def)
            .map(|t| t.generics.clone())
            .unwrap_or_default();
        let subst: HashMap<DefId, Ty> = generics.iter().map(|g| (*g, self.fresh())).collect();
        let args = generics.iter().map(|g| subst[g].clone()).collect();
        (Ty::Adt(def, args), subst)
    }

    /// `instance`, with the type arguments written at the construction
    /// (`Box<Shout>(raw: …)`) in place of fresh ones, so a parameter no
    /// field mentions still gets its type.
    pub(crate) fn written_instance(
        &mut self,
        def: DefId,
        written: &[kavach_ast::TypeExpr],
        span: kavach_ast::Span,
    ) -> (Ty, HashMap<DefId, Ty>) {
        let (instance, subst) = self.instance(def);
        let Ty::Adt(_, args) = &instance else {
            return (instance, subst);
        };
        if !written.is_empty() && written.len() != args.len() {
            let name = self.res.defs[def].name.clone();
            let takes = match args.len() {
                1 => "1 type argument".to_string(),
                n => format!("{n} type arguments"),
            };
            self.error(
                span,
                format!("`{name}` takes {takes}, not {}", written.len()),
            );
        } else {
            for (arg, w) in args.clone().iter().zip(written) {
                let t = self.lower(w, None);
                self.inf.unify(arg, &t);
            }
        }
        (instance, subst)
    }
}
