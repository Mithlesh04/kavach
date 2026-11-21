// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The parts of a spawned call worked out before its task starts (spec
//! 9.1): its arguments, the value a method is called on, and a function
//! worked out from an expression, as Go's `go f(x)` does. The interpreter and code generation both read the
//! parts found here, so they work out the same ones.
//!
//! A part whose value is the same wherever it is worked out (a name, a
//! literal, a lambda, a field path the method is called on) is left to
//! the task: it can't fail, panic or change anything, and the task gets
//! the variable it names anyway.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::{Expr, ExprKind, StrPart};
use std::collections::HashSet;

/// What finding the parts needs, kept while a program is checked.
#[derive(Default)]
pub(crate) struct SpawnParts {
    /// The arguments given to `inout` parameters (spec 3.9): places, not
    /// values, so the task works them out.
    pub(crate) inout_args: HashSet<(u32, u32)>,
    /// The parts worked out at the spawn, by span.
    pub(crate) parts: HashSet<(u32, u32)>,
}

/// The built-in methods that change the list, dict or set they are called
/// on (spec 2.2): it is a place, not a value.
const CHANGING: &[&str] = &[
    "push",
    "pop",
    "swap",
    "add",
    "remove",
    "insert",
    "replace",
    "clear",
    "push_char",
];

impl Checker<'_> {
    /// Notes the parts of spawned `x` worked out before its task starts.
    /// `x` that is not a call runs whole in the task, and so does a call
    /// through `?.` (its arguments may never be worked out, 2.7) and a
    /// wait with a time limit, `w.timeout(d)` (9.6).
    pub(crate) fn note_spawn_parts(&mut self, x: &Expr) {
        let ExprKind::Call { callee, args, .. } = &x.kind else {
            return;
        };
        let mut parts: Vec<&Expr> = Vec::new();
        if let ExprKind::Field {
            base,
            name,
            optional,
        } = &callee.kind
        {
            let wait = name.name == "timeout" && matches!(base.kind, ExprKind::Call { .. });
            if *optional || wait {
                return;
            }
            if !same_anywhere(base) && !field_path(base) && !self.changes_receiver(base, &name.name)
            {
                parts.push(base);
            }
        } else if !same_anywhere(callee) {
            // A function worked out from an expression, first (9.1).
            parts.push(callee);
        }
        for a in args {
            let span = (a.value.span.start, a.value.span.end);
            if !same_anywhere(&a.value) && !self.spawns.inout_args.contains(&span) {
                parts.push(&a.value);
            }
        }
        for p in parts {
            self.spawns.parts.insert((p.span.start, p.span.end));
        }
    }

    /// Whether `base.name(…)` is a built-in method that changes `base`, a
    /// list, dict, set, ring or text builder.
    fn changes_receiver(&self, base: &Expr, name: &str) -> bool {
        if !CHANGING.contains(&name) {
            return false;
        }
        let span = base.span;
        let ty = self.expr_types.iter().rev().find(|(s, _)| *s == span);
        let ty = ty.map(|(_, t)| self.inf.resolve(t));
        matches!(
            ty,
            Some(Ty::List(_) | Ty::Map(..) | Ty::Set(_) | Ty::Builtin("Ring" | "TextBuilder", _))
        )
    }
}

/// Whether `e` gives the same value wherever it is worked out: a name, a
/// literal or a lambda.
fn same_anywhere(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Str(parts) => parts.iter().all(|p| matches!(p, StrPart::Text(_))),
        ExprKind::Int(_)
        | ExprKind::BigInt(_)
        | ExprKind::Float(_)
        | ExprKind::Char(_)
        | ExprKind::Bool(_)
        | ExprKind::None
        | ExprKind::Name(_)
        | ExprKind::SelfValue
        | ExprKind::It
        | ExprKind::Lambda { .. }
        | ExprKind::DotLambda(_) => true,
        _ => false,
    }
}

/// Whether `e` is `a.b`, or a longer path of fields, on a name or `self`:
/// reading it can't fail or change anything.
fn field_path(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Field {
            base,
            optional: false,
            ..
        } => matches!(base.kind, ExprKind::Name(_) | ExprKind::SelfValue) || field_path(base),
        _ => false,
    }
}
