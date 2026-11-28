// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What a `match`'s arms cover between them, looking inside nested
//! variants and records (spec 4.3): `Full(s: Circle(r))` and
//! `Full(s: Square(x))` cover `Full`, and `Customer(name)` covers
//! `Customer`.

use crate::checker::{Checker, FieldInfo, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// A `match` must cover every possible case (spec 4.3).
    pub(crate) fn check_coverage(&mut self, arms: &[MatchArm], ty: &Ty, span: Span) {
        let open: Vec<&Pattern> = arms
            .iter()
            .filter(|a| a.guard.is_none())
            .map(|a| &a.pattern)
            .collect();
        if open.iter().any(|p| irrefutable(p)) {
            return;
        }
        if matches!(self.inf.resolve(ty), Ty::Builtin("Error", _)) {
            self.error(
                span,
                "a `match` on an `Error` needs a `_ => …` arm: an `Error` can hold any error type",
            );
            return;
        }
        let union_missing = |def: DefId, pats: &[&Pattern]| -> Vec<String> {
            match self.types.get(&def).map(|t| &t.shape) {
                // Named as a case is written: with the union (decision 0070).
                Some(Shape::Union(vs)) => vs
                    .iter()
                    .filter(|v| !self.variant_covered(pats, def, v.def, 0))
                    .flat_map(|v| self.missing_in(pats, def, v.def, 0))
                    .collect(),
                // Every value of a record fits its pattern's shape.
                Some(Shape::Record(fs)) if self.fields_covered(pats, def, fs, 0) => Vec::new(),
                _ => vec!["every other value (add `_ => …`)".to_string()],
            }
        };
        let missing: Vec<String> = match self.inf.resolve(ty) {
            Ty::Adt(def, _) => union_missing(def, &open),
            Ty::Prim(Prim::Bool) => ["true", "false"]
                .into_iter()
                .filter(|b| !open.iter().any(|p| matches!(&p.kind, PatternKind::Literal(Expr { kind: ExprKind::Bool(v), .. }) if v.to_string() == *b)))
                .map(String::from)
                .collect(),
            // `none` and the present values: a name, or patterns that cover
            // the inner type, such as each variant of a union (C9).
            Ty::Optional(inner) => {
                let has_none = open.iter().any(|p| matches!(p.kind, PatternKind::None));
                let present: Vec<&Pattern> = (open.iter().copied())
                    .filter(|p| !matches!(p.kind, PatternKind::None))
                    .collect();
                let mut missing = Vec::new();
                if !has_none {
                    missing.push("none".to_string());
                }
                if !self.pats_cover(&present, &inner, 0) {
                    missing.push("a present value (`value => …`)".to_string());
                }
                missing
            }
            Ty::Result(ok, err) => {
                let mut missing = Vec::new();
                let oks: Vec<&Pattern> = open.iter().filter_map(|p| match &p.kind { PatternKind::Ok(i) => Some(&**i), _ => None }).collect();
                if !self.pats_cover(&oks, &ok, 0) {
                    missing.push("ok(…)".to_string());
                }
                let errs: Vec<&Pattern> = open.iter().filter_map(|p| match &p.kind { PatternKind::Err(i) => Some(&**i), _ => None }).collect();
                if !errs.iter().any(|p| irrefutable(p)) {
                    match self.inf.resolve(&err) {
                        Ty::Adt(def, _) => missing.extend(union_missing(def, &errs).into_iter().map(|v| format!("err({v})"))),
                        _ => missing.push("err(…)".to_string()),
                    }
                }
                missing
            }
            Ty::Error | Ty::Var(_) => Vec::new(),
            _ => vec!["every other value (add `_ => …`)".to_string()],
        };
        if !missing.is_empty() {
            let list = missing
                .iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ");
            self.error(span, format!("this `match` does not cover {list}"));
        }
    }

    /// Whether `pats` together match every value of `variant` of union
    /// `union`.
    pub(crate) fn variant_covered(
        &self,
        pats: &[&Pattern],
        union: DefId,
        variant: DefId,
        depth: usize,
    ) -> bool {
        let fields = self.variant_fields(union, variant).unwrap_or_default();
        self.fields_covered(pats, variant, &fields, depth)
    }

    /// Whether `pats` together match every value of `def`, a variant or a
    /// record whose fields are `fields`: one arm takes any value of each
    /// field, or the arms differ in one field only and cover that field's
    /// type between them.
    fn fields_covered(
        &self,
        pats: &[&Pattern],
        def: DefId,
        fields: &[FieldInfo],
        depth: usize,
    ) -> bool {
        let arms = self.arms_of(pats, def);
        let whole = arms
            .iter()
            .any(|fs| (fs.iter().enumerate()).all(|(k, f)| self.takes_any(k, f, fields, depth)));
        if whole {
            return true;
        }
        if depth > 8 {
            return false;
        }
        (self.told_apart(&arms, fields, depth))
            .is_some_and(|(_, inner, ty)| self.pats_cover(&inner, &ty, depth + 1))
    }

    /// Whether field pattern `f`, the `k`th, takes any value of its field:
    /// a name or `_`, or a pattern that covers the field's type, such as a
    /// record pattern of names (`item: Point(x, y)`).
    fn takes_any(&self, k: usize, f: &FieldPattern, fields: &[FieldInfo], depth: usize) -> bool {
        irrefutable(&f.pattern)
            || (depth <= 8
                && field_of(k, f, fields)
                    .is_some_and(|i| self.pats_cover(&[&f.pattern], &i.ty, depth + 1)))
    }

    /// The arms of one variant or record type, as their field patterns.
    fn arms_of<'p>(&self, pats: &[&'p Pattern], def: DefId) -> Vec<&'p [FieldPattern]> {
        (pats.iter())
            .filter_map(|p| match &p.kind {
                PatternKind::Variant { path, fields } => {
                    let last = path.segments.last()?;
                    (self.pattern_variants.get(&last.span.start) == Some(&def))
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
        fields: &[FieldInfo],
        depth: usize,
    ) -> Option<(String, Vec<&'p Pattern>, Ty)> {
        let mut tested = (arms.iter().flat_map(|fs| fs.iter().enumerate()))
            .filter(|(k, f)| !self.takes_any(*k, f, fields, depth))
            .map(|(k, f)| field_of(k, f, fields).map(|i| i.name.clone()));
        let field = tested.next()??;
        if tested.any(|n| n.as_ref() != Some(&field)) {
            return None;
        }
        let info = fields.iter().find(|f| f.name == field)?;
        let inner: Vec<&Pattern> = (arms.iter())
            .flat_map(|fs| fs.iter().enumerate())
            .filter(|(k, f)| field_of(*k, f, fields).is_some_and(|i| i.name == field))
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
        let fields = self.variant_fields(union, variant).unwrap_or_default();
        let inside = (depth < 8)
            .then(|| self.told_apart(&arms, &fields, depth))
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
                Some(Shape::Record(fs)) => self.fields_covered(pats, def, fs, depth),
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

/// The field that field pattern `f`, the `k`th, takes: the one it names,
/// or the one at its place.
fn field_of<'f>(k: usize, f: &FieldPattern, fields: &'f [FieldInfo]) -> Option<&'f FieldInfo> {
    match &f.name {
        Some(n) => fields.iter().find(|i| i.name == n.name),
        None => fields.get(k),
    }
}

/// A pattern that matches every value.
fn irrefutable(p: &Pattern) -> bool {
    match &p.kind {
        PatternKind::Wildcard | PatternKind::Binding(_) => true,
        PatternKind::Tuple(items) => items.iter().all(irrefutable),
        _ => false,
    }
}
