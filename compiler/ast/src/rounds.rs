// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! What the rounds of an `on threads` loop change outside themselves
//! (spec 4.9): each writes its own slot of some lists (`out[i] = …`) and
//! adds into some whole-number `let`s (`total += n`). The checker allows
//! nothing else; the interpreter and native code run each round as a task
//! and apply these when it is waited for, in round order.

use crate::visit::{VisitMut, walk_expr, walk_pattern, walk_stmt};
use crate::*;

/// How a round adds into an outside variable.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundSum {
    /// The variable, as the assignment writes it: `total`.
    pub target: Expr,
    /// `total = (total + n) % m`: the sum is kept modulo `m`.
    pub modulo: Option<u64>,
}

/// What the rounds of a loop over `body` change outside it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoundWrites {
    /// The own slot each round writes, as the assignment writes it:
    /// `out[i]`, once per list.
    pub slots: Vec<Expr>,
    pub sums: Vec<RoundSum>,
    /// The body can leave the loop or the function (`break`, `return`,
    /// `fail`, `try`): its rounds then run one after another.
    pub leaves: bool,
}

/// The writes of `body`, a loop whose round has the name `slot` (`i` in
/// `for i in …`).
pub fn round_writes(body: &Block, slot: Option<&str>) -> RoundWrites {
    let mut body = body.clone();
    let mut v = Finder {
        slot: slot.map(str::to_string),
        inside: Vec::new(),
        lambdas: 0,
        found: RoundWrites::default(),
    };
    // Names declared in the body belong to the round.
    let mut names = Names(Vec::new());
    crate::visit::walk_block(&mut names, &mut body);
    v.inside = names.0;
    crate::visit::walk_block(&mut v, &mut body);
    v.found
}

struct Names(Vec<String>);

impl VisitMut for Names {
    fn pattern(&mut self, p: &mut Pattern) {
        if let PatternKind::Binding(id) = &p.kind {
            self.0.push(id.name.clone());
        }
        walk_pattern(self, p);
    }

    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Lambda { params, .. } = &e.kind {
            self.0.extend(params.iter().map(|p| p.name.name.clone()));
        }
        walk_expr(self, e);
    }
}

struct Finder {
    slot: Option<String>,
    inside: Vec<String>,
    lambdas: u32,
    found: RoundWrites,
}

impl Finder {
    fn outside(&self, name: &str) -> bool {
        !self.inside.iter().any(|n| n == name)
    }
}

impl VisitMut for Finder {
    fn stmt(&mut self, s: &mut Stmt) {
        if let StmtKind::Assign { target, op, value } = &s.kind {
            match &target.kind {
                ExprKind::Index { base, index } if matches!(&index.kind, ExprKind::Name(i) if Some(&i.name) == self.slot.as_ref()) => {
                    if let ExprKind::Name(list) = &base.kind
                        && self.outside(&list.name)
                        && !self
                            .found
                            .slots
                            .iter()
                            .any(|t| root_name(t) == Some(&list.name))
                    {
                        self.found.slots.push(target.clone());
                    }
                }
                ExprKind::Name(t) if self.outside(&t.name) => {
                    let modulo = match (&value.kind, op) {
                        (ExprKind::Binary(BinaryOp::Rem, _, m), AssignOp::Set) => match m.kind {
                            ExprKind::Int(n) => Some(n),
                            _ => None,
                        },
                        _ => None,
                    };
                    if !self
                        .found
                        .sums
                        .iter()
                        .any(|x| root_name(&x.target) == Some(&t.name))
                    {
                        self.found.sums.push(RoundSum {
                            target: target.clone(),
                            modulo,
                        });
                    }
                }
                _ => {}
            }
        }
        walk_stmt(self, s);
    }

    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Lambda { .. } | ExprKind::DotLambda(_) => {
                // A lambda's `return` is its own.
                self.lambdas += 1;
                walk_expr(self, e);
                self.lambdas -= 1;
                return;
            }
            ExprKind::Break(_)
            | ExprKind::Continue(Some(_))
            | ExprKind::Return(_)
            | ExprKind::Fail(_)
            | ExprKind::Try { .. }
                if self.lambdas == 0 =>
            {
                self.found.leaves = true;
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}

/// The variable a target writes: `out` in `out[i]`, `total`.
pub fn root_name(target: &Expr) -> Option<&String> {
    match &target.kind {
        ExprKind::Name(id) => Some(&id.name),
        ExprKind::Index { base, .. } => root_name(base),
        _ => None,
    }
}
