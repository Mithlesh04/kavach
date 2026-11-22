// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Distinct types (spec 2.21): a value of `type NetId = distinct Int` is
//! the `Int` it holds, as in native code, so it prints, compares, sorts
//! and is written to JSON as that value. `NetId(7)` gives the 7, and
//! `n.value`, read or changed, is `n` itself.

use crate::interp::Interp;
use kavach_ast::{Expr, TypeBody};
use kavach_checker::Ty;
use kavach_resolver::DefId;

impl<'a> Interp<'a> {
    /// Whether `def` is a distinct type.
    pub(crate) fn distinct_def(&self, def: DefId) -> bool {
        self.records
            .get(&def)
            .is_some_and(|t| matches!(&t.body, TypeBody::Record(r) if r.distinct))
    }

    /// Whether `e`'s value is of a distinct type, so its `.value` is
    /// the value itself.
    pub(crate) fn distinct_expr(&self, e: &Expr) -> bool {
        matches!(
            self.c.expr_types.get(&(e.span.start, e.span.end)),
            Some(Ty::Adt(def, _)) if self.distinct_def(*def)
        )
    }
}
