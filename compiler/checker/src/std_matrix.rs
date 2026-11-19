// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The signatures of `std.matrix`'s built-in functions (spec 2.17,
//! decision 0111): the work on a matrix's flat list of items, written
//! `matrix.product_id<T>(…)` inside `Matrix<T>`'s own methods, so the item
//! type comes as a type argument.

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
        let int = Ty::Prim(Prim::Int);
        let items = Ty::List(Box::new(t.clone()));
        let ints = Ty::List(Box::new(int.clone()));
        let p = |n: &str, t: &Ty| (n.to_string(), t.clone(), false);
        let (a, b) = (p("a", &items), p("b", &items));
        Some(match name {
            // The sizes: rows, inner, columns, and 1 for `fast`.
            "product_id" => (vec![a, b, p("sizes", &ints)], items),
            // 0 add, 1 subtract, 2 multiply.
            "zip_id" => (vec![a, b, p("op", &int)], items),
            "scale_id" => (vec![a, p("x", &t)], items),
            "negate_id" => (vec![a], items),
            // The sizes: rows, columns, and for sums 0 all, 1 rows, 2 columns.
            "transpose_id" | "sums_id" => (vec![a, p("sizes", &ints)], items),
            _ => return None,
        })
    }
}
