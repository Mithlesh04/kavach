// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Distinct types (spec 2.21): `type NetId = distinct Int` is lowered as
//! the `Int` it holds, so it costs nothing. `NetId(7)` is the 7, and
//! `n.value`, read or changed, is `n` itself; printing, comparing,
//! hashing and JSON work on the held value as they do on any other.

use crate::lower::Lowerer;
use kavach_ast::{self as ast, ItemKind, TypeBody};
use kavach_checker::Ty;
use kavach_resolver::DefId;

impl<'a> Lowerer<'a> {
    /// The declaration of distinct type `def`, if it is one.
    pub(crate) fn distinct_decl(&self, def: DefId) -> Option<&'a ast::RecordBody> {
        let decl_span = self.c.resolution.defs.get(def)?.span?;
        self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(t) if t.name.span == decl_span => match &t.body {
                TypeBody::Record(r) if r.distinct => Some(r),
                _ => None,
            },
            _ => None,
        })
    }

    /// Whether `e`'s value is of a distinct type, so its `.value` is the
    /// value itself.
    pub(crate) fn distinct_expr(&self, e: &ast::Expr) -> bool {
        matches!(
            self.c.expr_types.get(&(e.span.start, e.span.end)),
            Some(Ty::Adt(def, _)) if self.distinct_decl(*def).is_some()
        )
    }
}
