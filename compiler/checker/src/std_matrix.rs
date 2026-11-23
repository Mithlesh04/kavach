// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The signature of `matrix.product_id<T>(a, b, sizes)`, the product of
//! `std.tensor`'s `a @ b` on flat lists of items (spec 14.33, decisions
//! 0111 and 0136). The item type comes as a type argument. Only the
//! standard library may `use std.matrix` (`stdlib.rs`).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Span, TypeExpr};

type Signature = (Vec<(String, Ty, bool)>, Ty);

impl Checker<'_> {
    /// The parameters and result of `matrix.<name>`, or `None`.
    pub(crate) fn matrix_signature(
        &mut self,
        name: &str,
        generics: &[TypeExpr],
        span: Span,
    ) -> Option<Signature> {
        if !name.ends_with("_id") {
            return None;
        }
        let Some(t) = generics.first().map(|g| self.lower(g, None)) else {
            self.error(
                span,
                format!("`matrix.{name}` needs its item type: `matrix.{name}<T>(…)`"),
            );
            return Some((Vec::new(), Ty::Error));
        };
        let items = Ty::List(Box::new(t));
        let ints = Ty::List(Box::new(Ty::Prim(Prim::Int)));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let (a, b) = (p("a", &items), p("b", &items));
        // The sizes: rows, inner, columns, and 1 for `fast`.
        (name == "product_id").then(|| (vec![a, b, p("sizes", &ints)], items))
    }

    /// `use std.matrix` outside the standard library: an error, as
    /// `std.matrix` is gone (spec 2.17). Whether it was one.
    pub(crate) fn matrix_gone(&mut self, module: &str, span: Span) -> bool {
        let file = &self.res.modules[self.res.module_at(span.start)].name;
        if module != "matrix" || file.starts_with("std.") {
            return false;
        }
        self.error(span, "`std.matrix` is gone: use `std.tensor`, where a matrix is a `Tensor<T, dims: 2>` (spec 2.17)");
        true
    }
}
