// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Grouping and statistics (spec 14.2, decision 0135). The methods are
//! written in Kavach (`stdlib/collections/grouping.kav` and `stats.kav`);
//! the checker gives `mean` and `median` their result type, and names
//! Kavach's method where a program writes another tool's.

use crate::checker::{Checker, FnSig};
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, ExprKind, Ident};

/// The module that writes `mean` and `median`.
const STATS: &str = "std.collections.stats";

/// Kavach's name for a job on a list that other tools name differently:
/// pandas' `value_counts` is `count_by`. For "has no method".
pub(crate) fn table_name(recv: &Ty, name: &str) -> Option<&'static str> {
    if !matches!(recv, Ty::List(_)) {
        return None;
    }
    Some(match name {
        "sum_by" | "sumBy" => {
            "group_by(key).map_values(g => g.sum(f))` for a sum per key, or `sum(f)"
        }
        "value_counts" | "countBy" => "count_by(key)",
        "groupBy" => "group_by(key)",
        "distinct" | "drop_duplicates" | "uniq" => "unique()` or `unique_by(key)",
        "minBy" => "min_by(f)",
        "maxBy" => "max_by(f)",
        "merge" => {
            "inner_join(other, key, other_key, row)` or `left_join(other, key, other_key, row)"
        }
        "select" => "map(x => (a: x.a, b: x.b))",
        "average" | "avg" => "mean(f)",
        _ => return None,
    })
}

/// What `std.math` and `std.collections` had that is now a list method.
pub(crate) fn now_a_method(module: &str, name: &str) -> Option<String> {
    let method = match (module, name) {
        ("math", "mean") => "xs.mean(x => x)",
        ("collections", "unique") => "items.unique()",
        ("collections", "counts") => "items.count_by(x => x)",
        _ => return None,
    };
    Some(format!(
        "`{module}.{name}` is gone: write the list method `{method}` (spec 14.2, decision 0135)"
    ))
}

impl Checker<'_> {
    /// `orders.join(users, …)`: `join` joins text; a join of two lists
    /// on a key is `inner_join` or `left_join` (spec 14.2).
    pub(crate) fn join_given_lambdas(
        &mut self,
        recv: &Ty,
        name: &Ident,
        args: &[Arg],
    ) -> Option<Ty> {
        let lambda = |a: &Arg| {
            matches!(
                a.value.kind,
                ExprKind::Lambda { .. } | ExprKind::DotLambda(_)
            )
        };
        if !matches!(recv, Ty::List(_)) || name.name != "join" {
            return None;
        }
        if args.len() < 2 && !args.iter().any(lambda) {
            return None;
        }
        for a in args {
            self.value(&a.value, None);
        }
        let shown = self.show(recv);
        self.error(name.span, format!("`{shown}`'s `join` joins text: Kavach writes a join on a key as `inner_join(other, key, other_key, row)`, or `left_join` to keep the items that match nothing (spec 14.2)"));
        Some(Ty::Error)
    }

    /// `xs.mean(f)` and `xs.median(f)` (spec 14.2) give a `Float` for
    /// whole numbers and `Float`, a `Float32` for `Float32` and a
    /// `Decimal` for `Decimal`. `std.collections.stats` writes the result
    /// as a type parameter, which this settles from what `f` gives.
    pub(crate) fn settle_stats(&mut self, name: &str, sig: &FnSig, args: &[Arg], ret: &Ty) {
        if !matches!(name, "mean" | "median") {
            return;
        }
        let Some(span) = sig.def.and_then(|d| self.res.defs[d].span) else {
            return;
        };
        // `std.table`'s `mean` and `median` give what a list's give (14.34).
        if !matches!(
            self.res.modules[self.res.module_at(span.start)]
                .name
                .as_str(),
            STATS | "std.table"
        ) {
            return;
        }
        let Some(arg) = args.first() else {
            return;
        };
        let Some(Ty::Fn(_, gives, _)) = self.type_at(arg.value.span).map(|t| self.inf.resolve(&t))
        else {
            return;
        };
        let result = match self.inf.resolve(&gives) {
            Ty::Prim(p @ (Prim::Float32 | Prim::Decimal)) => Ty::Prim(p),
            // A generic number: the call's own type arguments say.
            Ty::Param(_) | Ty::Error => return,
            _ => Ty::Prim(Prim::Float),
        };
        self.inf.unify(ret, &Ty::Optional(Box::new(result)));
    }

    /// `xs.sum(f)` adds each item's number (spec 14.2): what `f` gives
    /// must be a number type, as a `Number` bound asks. A distinct type
    /// has no arithmetic (2.21), so its `.value` is summed instead.
    pub(crate) fn sum_gives_a_number(&mut self, ret: &Ty, span: kavach_ast::Span) {
        // A function that can fail passes its error on (spec 5.2).
        let gives = &match self.inf.shallow(ret) {
            Ty::Result(ok, _) => *ok,
            _ => ret.clone(),
        };
        let ok = match self.inf.shallow(gives) {
            Ty::Prim(p) => p.is_numeric(),
            Ty::Param(_) => self.number_param(gives, false),
            // Not known yet: it may only become a number.
            Ty::Var(_) => {
                let number = self.inf.fresh(crate::infer::VarKind::IntLiteral);
                self.inf.unify(gives, &number)
            }
            Ty::Error | Ty::Never => true,
            _ => false,
        };
        if ok {
            return;
        }
        let shown = self.show(gives);
        let message = match self.distinct_held(gives) {
            Some(_) => format!(
                "`sum` adds numbers, and `{shown}` is a distinct type, which has no arithmetic: sum its value, as `xs.sum(x => x.value)` (spec 2.21)"
            ),
            None => format!(
                "`sum` adds numbers, and the function gives `{shown}`: give each item's number, as `words.sum(w => w.len())` (spec 14.2)"
            ),
        };
        self.error(span, message);
    }

    /// A method added to a collection twice (spec 2.5): a program's own
    /// `extend<T> [T] { fn mean … }` takes the name of a method the
    /// standard library adds (14.2). The error is at the program's; the
    /// standard library's is kept. Whether to leave `sig` out.
    pub(crate) fn added_twice(&mut self, kind: &str, name: &str, sig: &FnSig, target: &Ty) -> bool {
        let Some(old) = self
            .ext_methods
            .get(kind)
            .and_then(|m| m.get(name))
            .cloned()
        else {
            return false;
        };
        let span_of = |s: &FnSig| s.def.and_then(|d| self.res.defs[d].span);
        let from_std = |at: Option<kavach_ast::Span>| {
            at.is_some_and(|at| {
                self.res.modules[self.res.module_at(at.start)]
                    .name
                    .starts_with("std.")
            })
        };
        let new_is_std = from_std(span_of(sig));
        let at = if new_is_std {
            span_of(&old)
        } else {
            span_of(sig)
        };
        if let Some(at) = at {
            let shown = self.show(target);
            self.error(
                at,
                format!(
                    "`{shown}` already has a method `{name}`: give yours another name (spec 2.5)"
                ),
            );
        }
        !new_is_std
    }
}
