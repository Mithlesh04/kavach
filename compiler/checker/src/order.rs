// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Which values have an order (spec 1.8), for `<`, `sort`, `min`, `max`
//! and `sort_by`'s key; and the waits refused inside `Shared.update`
//! (spec 6.2).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::Span;

fn number(t: &Ty) -> bool {
    matches!(t, Ty::Prim(p) if p.is_int() || matches!(p, Prim::Float | Prim::Float32 | Prim::Decimal | Prim::BigInt))
}

/// Values with an order (spec 1.8), for `<`, `sort`, `min`, `max` and
/// the key of `sort_by`: numbers, text, characters, `Bool` (`false`
/// first), text views and `Bytes`, and tuples, lists and rings of them,
/// item by item.
fn orderable(t: &Ty) -> bool {
    number(t)
        || matches!(
            t,
            Ty::Prim(Prim::String | Prim::Char | Prim::Bool | Prim::Bytes)
        )
        || matches!(t, Ty::Builtin("StrView", _))
        || matches!(t, Ty::Tuple(items) if items.iter().all(orderable))
        || matches!(t, Ty::List(item) if orderable(item))
        || matches!(t, Ty::Builtin("Ring", items) if items.iter().all(orderable))
}

impl Checker<'_> {
    /// `what` waits: an error inside the function given to `update`,
    /// which runs with the `Shared` value locked (spec 6.2, decision 0052).
    pub(crate) fn no_wait_in_update(&mut self, span: Span, what: &str) {
        if self.updating > 0 {
            self.error(
                span,
                format!("{what} waits, and the function given to `update` can't wait: the `Shared` value stays locked until it returns. Get what the update needs first, then update (spec 6.2)"),
            );
        }
    }

    /// Whether values of `t` have an order; `None` while `t` isn't known.
    pub(crate) fn ordered(&self, t: &Ty) -> Option<bool> {
        let t = self.inf.resolve(t);
        // A `T: Number` has the order of numbers (spec 2.9).
        if self.number_param(&t, false) {
            return Some(true);
        }
        let unknown = |t: &Ty| matches!(t, Ty::Var(_) | Ty::Error);
        let open = match &t {
            Ty::Tuple(items) => items.iter().any(unknown),
            Ty::List(item) => unknown(item),
            other => unknown(other),
        };
        (!open).then(|| orderable(&t))
    }

    /// Reports a value without an order where one is needed.
    pub(crate) fn require_order(&mut self, t: &Ty, span: Span, what: &str) {
        if self.ordered(t) == Some(false) && !self.number_param(t, false) {
            let shown = self.show(t);
            // A set or a map: its items, sorted, have one (spec 1.8).
            let hint = match self.inf.resolve(t) {
                Ty::Set(_) => ": compare `s.to_list().sort()` instead",
                Ty::Map(..) => ": compare `m.keys().sort()`, or its entries, instead",
                _ => "",
            };
            self.error(
                span,
                format!("{what} needs values with an order (numbers, text, characters, `Bool`, `Bytes`, text views, or tuples, lists and rings of them), and `{shown}` has none{hint}"),
            );
        }
    }
}
