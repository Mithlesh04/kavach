//! A `for` over a range the compiler knows (spec 1.8): its body's statements
//! that always run see the loop's values, as the run will. If the first or
//! the last value panics, the program always panics, and the first round
//! that does gives the message. Only a body that can't leave early counts:
//! a `break` or a `try` may be what keeps the last round from running.

use super::Holds;
use super::lets::mentions_set;
use super::{BinaryOp, Known};
use crate::constants::fits;
use kavach_ast::visit::{VisitMut, walk_expr};
use kavach_ast::*;
use kavach_resolver::DefId;
use num_bigint::BigInt;

/// How far to look for the first round that panics, when the last does.
const MOST_ROUNDS_TRIED: u32 = 1_000_000;

/// The first and last value of a `for`, and its variable if it has one.
pub(super) struct Rounds {
    pub(super) var: Option<DefId>,
    pub(super) first: BigInt,
    pub(super) last: BigInt,
}

/// Whether what it visits can leave a loop's body early.
#[derive(Default)]
struct Leaves(bool);

impl VisitMut for Leaves {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Lambda { .. } | ExprKind::DotLambda(_) => {}
            ExprKind::Break(_)
            | ExprKind::Continue(_)
            | ExprKind::Return(_)
            | ExprKind::Fail(_)
            | ExprKind::Try { .. } => self.0 = true,
            _ => walk_expr(self, e),
        }
    }
}

impl Known<'_, '_> {
    /// The rounds of `for pattern in iter { body }`, when its range is known
    /// and holds something, and its body can't leave early.
    pub(super) fn known_rounds(
        &self,
        pattern: &Pattern,
        iter: &Expr,
        body: &Block,
    ) -> Option<Rounds> {
        let ExprKind::Binary(op @ (BinaryOp::Range | BinaryOp::RangeInclusive), from, to) =
            &iter.kind
        else {
            return None;
        };
        let (first, end) = (self.value(from)?, self.value(to)?);
        let last = match op {
            BinaryOp::Range => end - 1,
            _ => end,
        };
        if first > last {
            return None;
        }
        let mut leaves = Leaves::default();
        for s in &mut body.clone().stmts {
            leaves.stmt(s);
        }
        if leaves.0 {
            return None;
        }
        let var = match &pattern.kind {
            PatternKind::Binding(id) => Some(*self.checker.decl.get(&id.span.start)?),
            PatternKind::Wildcard => None,
            _ => return None,
        };
        Some(Rounds { var, first, last })
    }

    /// Runs `check`, a check that reports what always panics, for the known
    /// values, and then for each loop around it: the first and the last
    /// value. When one reports, the first value that does gives the message.
    pub(super) fn each_round(&mut self, check: impl Fn(&mut Self)) {
        let errors = self.errors.len();
        check(self);
        let warnings = self.warnings.len();
        if self.errors.len() > errors || self.ranges.is_empty() {
            return;
        }
        let ranges: Vec<_> = self.ranges.iter().map(|(d, r)| (*d, r.clone())).collect();
        for (var, (first, last)) in ranges {
            let mut panics = false;
            for v in [&first, &last] {
                self.lets.insert(var, Holds::Whole(v.clone()));
                check(self);
                panics |= self.errors.len() > errors;
                self.errors.truncate(errors);
            }
            if panics {
                let mut v = first;
                for _ in 0..MOST_ROUNDS_TRIED {
                    self.lets.insert(var, Holds::Whole(v.clone()));
                    check(self);
                    if self.errors.len() > errors || v == last {
                        break;
                    }
                    v += 1;
                }
                if self.errors.len() == errors {
                    self.lets.insert(var, Holds::Whole(last));
                    check(self);
                }
            }
            self.lets.remove(&var);
            if panics {
                break;
            }
        }
        self.warnings.truncate(warnings);
    }

    /// `t += c` or `t -= c` among the statements of a known loop's body,
    /// with `t` and `c` known and nothing else changing them: whether the
    /// rounds carry `t` past its type (spec 1.8).
    pub(super) fn accumulators(&mut self, rounds: &Rounds, body: &Block) {
        let count = &rounds.last - &rounds.first + 1;
        let changed = mentions_set(self, body);
        for s in &body.stmts {
            let StmtKind::Assign {
                target,
                op: op @ (AssignOp::Add | AssignOp::Sub),
                value,
            } = &s.kind
            else {
                continue;
            };
            let ExprKind::Name(id) = &target.kind else {
                continue;
            };
            let Some(&t) = self.checker.res.uses.get(&id.span.start) else {
                continue;
            };
            let once = body.stmts.iter().filter(|o| {
                let one = Block {
                    stmts: vec![(*o).clone()],
                    span: o.span,
                };
                mentions_set(self, &one).contains(&t)
            });
            if once.count() != 1 {
                continue;
            }
            let mut read = super::lets::named_in(self, value);
            read.retain(|d| changed.contains(d));
            if !read.is_empty() {
                continue;
            }
            let (Some(start), Some(step), Some(p)) = (
                self.lets.get(&t).and_then(Holds::whole).cloned(),
                self.value(value),
                self.whole(target),
            ) else {
                continue;
            };
            let signed = if *op == AssignOp::Add {
                step.clone()
            } else {
                -step.clone()
            };
            let at = |k: &BigInt| &start + k * &signed;
            if !fits(&start, p) || fits(&at(&count), p) {
                continue;
            }
            // The last round before it leaves the type.
            let (mut fine, mut over) = (BigInt::from(0), count.clone());
            while &over - &fine > BigInt::from(1) {
                let middle: BigInt = (&fine + &over) / 2;
                if fits(&at(&middle), p) {
                    fine = middle;
                } else {
                    over = middle;
                }
            }
            let sign = if *op == AssignOp::Add { "+" } else { "-" };
            self.errors.push((
                s.span,
                format!(
                    "this always panics: integer overflow: {} {sign} {step} doesn't fit in {} (spec 1.8)",
                    at(&fine),
                    p.name()
                ),
            ));
        }
    }
}
