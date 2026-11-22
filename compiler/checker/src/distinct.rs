// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Distinct types (spec 2.21): `type NetId = distinct Int` is a record
//! whose one field, `value`, is what it holds, never mixed with another
//! type. It is made from its value alone, `NetId(7)`, compares as its
//! value does, and has no methods, interfaces or invariants of its own.

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::{File, ItemKind, TypeDecl, TypeKind};
use kavach_resolver::DefId;

/// Whether a distinct type may hold a value of type `t`: a built-in
/// number, `Bool`, `Char` or `String`.
fn can_hold(t: &Ty) -> bool {
    match t {
        Ty::Prim(p) => {
            p.is_int()
                || matches!(
                    p,
                    Prim::Float
                        | Prim::Float32
                        | Prim::Decimal
                        | Prim::BigInt
                        | Prim::Bool
                        | Prim::Char
                        | Prim::String
                )
        }
        // A sized or range number (2.1) is an `Int` underneath.
        Ty::Sized(inner, ..) => can_hold(inner),
        _ => false,
    }
}

impl Checker<'_> {
    /// Records `t` as distinct, refusing what it can't have.
    pub(crate) fn note_distinct(&mut self, def: DefId, t: &TypeDecl, held: &Ty) {
        self.distinct.insert(def);
        let name = &t.name.name;
        if !t.generics.is_empty() {
            self.error(t.name.span, format!("a distinct type has no type parameters: declare one for each type it holds, as `type {name} = distinct Int` (spec 2.21)"));
        }
        if let Some(i) = t.implements.first() {
            self.error(i.span, format!("a distinct type implements no interfaces yet: use `{name}.value` where one is needed (spec 2.21)"));
        }
        if !can_hold(held) && !matches!(held, Ty::Error) {
            let shown = self.show(held);
            self.error(t.name.span, format!("a distinct type holds a built-in number, `Bool`, `Char` or `String`, not `{shown}`: for more, declare a record (spec 2.21)"));
        }
    }

    /// The type a distinct type `t` holds, if `t` is one.
    pub(crate) fn distinct_held(&self, t: &Ty) -> Option<Ty> {
        let Ty::Adt(def, _) = self.inf.resolve(t) else {
            return None;
        };
        if !self.distinct.contains(&def) {
            return None;
        }
        match &self.types.get(&def)?.shape {
            Shape::Record(fields) => fields.first().map(|f| f.ty.clone()),
            _ => None,
        }
    }

    /// `extend` of a distinct type: it has no methods yet (spec 2.21).
    pub(crate) fn no_distinct_methods(&mut self, file: &File) {
        for item in &file.items {
            let ItemKind::Extend(e) = &item.kind else {
                continue;
            };
            let TypeKind::Named { path, .. } = &e.target.kind else {
                continue;
            };
            let def = path
                .segments
                .last()
                .and_then(|s| self.res.uses.get(&s.span.start));
            if def.is_some_and(|d| self.distinct.contains(d)) {
                let shown = &path.segments[path.segments.len() - 1].name;
                self.error(e.target.span, format!("a distinct type has no methods yet: write a function that takes a `{shown}` (spec 2.21)"));
            }
        }
    }
}
