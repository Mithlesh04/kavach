//! What a `match`'s arms cover between them, looking inside nested
//! variants (spec 4.3): `Full(s: Circle(r))` and `Full(s: Square(x))`
//! cover `Full`.

use crate::checker::{Checker, Shape};
use crate::patterns::irrefutable;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// Whether `pats` together match every value of `variant` of union
    /// `union`: one arm takes any value of each field, or the arms differ
    /// in one field only and cover that field's type between them.
    pub(crate) fn variant_covered(
        &self,
        pats: &[&Pattern],
        union: DefId,
        variant: DefId,
        depth: usize,
    ) -> bool {
        let named = &self.pattern_variants;
        let arms: Vec<&[FieldPattern]> = (pats.iter())
            .filter_map(|p| match &p.kind {
                PatternKind::Variant { path, fields } => {
                    let last = path.segments.last()?;
                    (named.get(&last.span.start) == Some(&variant))
                        .then_some(fields.as_deref().unwrap_or(&[]))
                }
                _ => None,
            })
            .collect();
        if arms
            .iter()
            .any(|fs| fs.iter().all(|f| irrefutable(&f.pattern)))
        {
            return true;
        }
        if depth > 8 {
            return false;
        }
        // The one field the arms tell apart, by name or by place.
        let fields = self.variant_fields(union, variant).unwrap_or_default();
        let key = |k: usize, f: &FieldPattern| -> Option<String> {
            (f.name.as_ref().map(|n| n.name.clone()))
                .or_else(|| fields.get(k).map(|i| i.name.clone()))
        };
        let mut tested = (arms.iter().flat_map(|fs| fs.iter().enumerate()))
            .filter(|(_, f)| !irrefutable(&f.pattern))
            .map(|(k, f)| key(k, f));
        let Some(Some(field)) = tested.next() else {
            return false;
        };
        if tested.any(|n| n.as_ref() != Some(&field)) {
            return false;
        }
        let Some(info) = fields.iter().find(|f| f.name == field) else {
            return false;
        };
        let inner: Vec<&Pattern> = (arms.iter())
            .flat_map(|fs| fs.iter().enumerate())
            .filter(|(k, f)| key(*k, f).as_ref() == Some(&field))
            .map(|(_, f)| &f.pattern)
            .collect();
        self.pats_cover(&inner, &info.ty, depth + 1)
    }

    /// Whether `pats` together match every value of type `ty`.
    pub(crate) fn pats_cover(&self, pats: &[&Pattern], ty: &Ty, depth: usize) -> bool {
        if pats.iter().any(|p| irrefutable(p)) {
            return true;
        }
        match self.inf.resolve(ty) {
            Ty::Adt(def, _) => match self.types.get(&def).map(|t| &t.shape) {
                Some(Shape::Union(vs)) => {
                    (vs.iter()).all(|v| self.variant_covered(pats, def, v.def, depth))
                }
                _ => false,
            },
            Ty::Prim(Prim::Bool) => ["true", "false"].iter().all(|b| {
                pats.iter().any(|p| {
                    matches!(&p.kind, PatternKind::Literal(Expr { kind: ExprKind::Bool(v), .. })
                        if v.to_string() == *b)
                })
            }),
            _ => false,
        }
    }
}
