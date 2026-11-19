// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
        let arms = self.arms_of(pats, variant);
        if arms
            .iter()
            .any(|fs| fs.iter().all(|f| irrefutable(&f.pattern)))
        {
            return true;
        }
        if depth > 8 {
            return false;
        }
        (self.told_apart(&arms, union, variant))
            .is_some_and(|(_, inner, ty)| self.pats_cover(&inner, &ty, depth + 1))
    }

    /// The arms of one variant, as their field patterns.
    fn arms_of<'p>(&self, pats: &[&'p Pattern], variant: DefId) -> Vec<&'p [FieldPattern]> {
        (pats.iter())
            .filter_map(|p| match &p.kind {
                PatternKind::Variant { path, fields } => {
                    let last = path.segments.last()?;
                    (self.pattern_variants.get(&last.span.start) == Some(&variant))
                        .then_some(fields.as_deref().unwrap_or(&[]))
                }
                _ => None,
            })
            .collect()
    }

    /// The one field `arms` tell apart, by name or by place: its name, the
    /// arms' patterns for it, and its type.
    fn told_apart<'p>(
        &self,
        arms: &[&'p [FieldPattern]],
        union: DefId,
        variant: DefId,
    ) -> Option<(String, Vec<&'p Pattern>, Ty)> {
        let fields = self.variant_fields(union, variant).unwrap_or_default();
        let key = |k: usize, f: &FieldPattern| -> Option<String> {
            (f.name.as_ref().map(|n| n.name.clone()))
                .or_else(|| fields.get(k).map(|i| i.name.clone()))
        };
        let mut tested = (arms.iter().flat_map(|fs| fs.iter().enumerate()))
            .filter(|(_, f)| !irrefutable(&f.pattern))
            .map(|(k, f)| key(k, f));
        let field = tested.next()??;
        if tested.any(|n| n.as_ref() != Some(&field)) {
            return None;
        }
        let info = fields.iter().find(|f| f.name == field)?;
        let inner: Vec<&Pattern> = (arms.iter())
            .flat_map(|fs| fs.iter().enumerate())
            .filter(|(k, f)| key(*k, f).as_ref() == Some(&field))
            .map(|(_, f)| &f.pattern)
            .collect();
        Some((field, inner, info.ty.clone()))
    }

    /// `variant` of `union` as a `match` misses it, named down to the
    /// inner variant the arms leave out: `Box.Full(s: Shape.Square(..))`.
    pub(crate) fn missing_in(
        &self,
        pats: &[&Pattern],
        union: DefId,
        variant: DefId,
        depth: usize,
    ) -> Vec<String> {
        let name = (self.types.get(&union).and_then(|t| match &t.shape {
            Shape::Union(vs) => vs.iter().find(|v| v.def == variant).map(|v| v.name.clone()),
            _ => None,
        }))
        .unwrap_or_default();
        let whole = format!("{}.{name}", self.def_name(union));
        let arms = self.arms_of(pats, variant);
        let inside = (depth < 8)
            .then(|| self.told_apart(&arms, union, variant))
            .flatten();
        let Some((field, inner, ty)) = inside else {
            return vec![whole];
        };
        let Ty::Adt(def, _) = self.inf.resolve(&ty) else {
            return vec![whole];
        };
        let Some(Shape::Union(vs)) = self.types.get(&def).map(|t| &t.shape) else {
            return vec![whole];
        };
        let out: Vec<String> = (vs.iter())
            .filter(|v| !self.variant_covered(&inner, def, v.def, depth + 1))
            .flat_map(|v| self.missing_in(&inner, def, v.def, depth + 1))
            .map(|m| format!("{whole}({field}: {m}(..))"))
            .collect();
        if out.is_empty() { vec![whole] } else { out }
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
