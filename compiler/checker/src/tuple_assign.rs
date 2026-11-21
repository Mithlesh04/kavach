// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Tuple assignment, `(a, b) = (b, a)` (spec 1.14): each place is checked
//! as `=` checks it alone, and the right side gives one fitting value for
//! each.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::*;

impl Checker<'_> {
    /// `(p, q) = value`, or one place of it given its own value.
    pub(crate) fn assign_tuple(&mut self, target: &Expr, value: &Expr) {
        let ExprKind::Tuple(places) = &target.kind else {
            let slot = self.tuple_place(target, value);
            self.check_expect(value, &slot);
            self.check_arena_store(target, value);
            return;
        };
        // `(a, b) = (b, a)`: each value goes with its place.
        if let ExprKind::Tuple(values) = &value.kind {
            if values.len() == places.len() {
                for (p, v) in places.iter().zip(values) {
                    self.assign_tuple(p, v);
                }
                return;
            }
            self.tuple_places(target, value);
            for v in values {
                self.value(v, None);
            }
            return self.count_mismatch(places.len(), values.len(), value.span);
        }
        // `(low, high) = bounds(xs)`: any value whose type is a tuple.
        let want = self.tuple_places(target, value);
        let got = self.value(value, Some(&want));
        match self.inf.resolve(&got) {
            Ty::Tuple(items) if items.len() != places.len() => {
                self.count_mismatch(places.len(), items.len(), value.span)
            }
            Ty::Tuple(_) | Ty::Var(_) => {
                self.fitting = Some(self.made(value));
                let fits = self.coerce(&got, &want);
                self.fitting = None;
                if !fits {
                    self.mismatch(value, &want, &got);
                }
            }
            Ty::Error | Ty::Never => {}
            _ => {
                let shown = self.show(&got);
                let n = places.len();
                self.error(value.span, format!("the right side is a `{shown}`, not a tuple: give one value for each of the {n} places (spec 1.14)"));
            }
        }
        for p in target.places() {
            self.check_arena_store(p, value);
        }
    }

    fn count_mismatch(&mut self, places: usize, values: usize, span: Span) {
        self.error(span, format!("the left side has {places} places and the right side {values} values: give one value for each place (spec 1.14)"));
    }

    /// The type `target` takes: its place's, or a tuple of its places'.
    fn tuple_places(&mut self, target: &Expr, value: &Expr) -> Ty {
        match &target.kind {
            ExprKind::Tuple(parts) => {
                Ty::Tuple(parts.iter().map(|p| self.tuple_places(p, value)).collect())
            }
            _ => self.tuple_place(target, value),
        }
    }

    /// One place a tuple assignment sets, checked as `place = value` is
    /// (`items.rs`): the type it takes.
    fn tuple_place(&mut self, target: &Expr, value: &Expr) -> Ty {
        let t = self.value(target, None);
        self.check_frozen_assign(target, AssignOp::Set);
        // `x[k] = v` on a program's own type becomes a call of its own,
        // which can't be one of several places (spec 2.16).
        if let ExprKind::Index { base, .. } = &target.kind
            && let Some(Ty::Adt(..)) = self.type_at(base.span).map(|b| self.inf.resolve(&b))
        {
            self.error(target.span, "an item of a program's own type can't be one of several places set at once yet: set it on its own line (spec 1.14)");
        }
        self.check_unsafe_index_target(target);
        self.check_slice_target(target, AssignOp::Set);
        self.check_round_assign(target, AssignOp::Set, value);
        self.check_invariant_in_loop(target);
        self.check_read_only_target(target, "assigned");
        self.check_bytes_unchanged(target);
        self.check_captured_change(target);
        self.map_slot(target).unwrap_or(t)
    }
}
