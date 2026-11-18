//! `on cores` and `on threads` before a `for` (spec 4.9): where a loop's
//! rounds run. An `on cores` loop must give the one-core result, so each
//! round may write only its own slot (`out[i]`) or add whole numbers into
//! an outside `let`, and may do no I/O; `on threads` allows I/O.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::DefId;

/// A loop with a place, while its body is checked.
#[derive(Clone)]
pub(crate) struct Round {
    /// The loop's body: names declared before it are outside the round.
    pub(crate) body: Span,
    /// The loop's own name (`i` in `for i in …`), for `out[i]`.
    pub(crate) name: Option<DefId>,
    /// `on threads`: I/O is allowed.
    pub(crate) threads: bool,
}

impl Checker<'_> {
    /// Checks a place's count; the round's rules apply while `f` checks
    /// the body.
    pub(crate) fn in_round(
        &mut self,
        place: &LoopPlace,
        (pattern, iter): (&Pattern, &Expr),
        body: &Block,
        f: impl FnOnce(&mut Self),
    ) {
        if let Some(n) = &place.count {
            self.check_expect(n, &Ty::Prim(Prim::Int));
            if matches!(n.kind, ExprKind::Int(0) | ExprKind::Unary(UnaryOp::Neg, _)) {
                let what = if place.threads { "thread" } else { "core" };
                self.error(
                    n.span,
                    format!("a loop runs on at least one {what}: give 1 or more (spec 4.9)"),
                );
            }
        }
        // The round's slot: the loop's name, or the index of
        // `for (i, x) in xs.enumerate()`.
        let slot = match (&pattern.kind, &iter.kind) {
            (PatternKind::Binding(id), _) => Some(id),
            (PatternKind::Tuple(ps), ExprKind::Call { callee, args, .. }) if args.is_empty() => {
                match (&ps[..], &callee.kind) {
                    ([first, _], ExprKind::Field { name, .. }) if name.name == "enumerate" => {
                        match &first.kind {
                            PatternKind::Binding(id) => Some(id),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let name = slot.and_then(|id| self.decl.get(&id.span.start).copied());
        self.rounds.push(Round {
            body: body.span,
            name,
            threads: place.threads,
        });
        f(self);
        self.rounds.pop();
    }

    /// Whether `e`'s root variable was declared before the innermost
    /// round: something the round shares with the others.
    fn outside_round(&self, e: &Expr) -> Option<(Round, String)> {
        let round = self.rounds.last()?.clone();
        let mut root = e;
        while let ExprKind::Field { base, .. }
        | ExprKind::Index { base, .. }
        | ExprKind::TupleField { base, .. } = &root.kind
        {
            root = base;
        }
        let ExprKind::Name(id) = &root.kind else {
            return None;
        };
        let def = *self.res.uses.get(&id.span.start)?;
        let span = self.res.defs[def].span?;
        (span.start < round.body.start || span.start >= round.body.end)
            .then(|| (round, id.name.clone()))
    }

    /// `target op= value` in a round: its own slot, or whole numbers added.
    pub(crate) fn check_round_assign(&mut self, target: &Expr, op: AssignOp, value: &Expr) {
        let Some((round, name)) = self.outside_round(target) else {
            return;
        };
        // `out[i] = …`: the round's own slot.
        if let ExprKind::Index { base, index } = &target.kind
            && matches!(base.kind, ExprKind::Name(_))
            && let ExprKind::Name(i) = &index.kind
            && round.name.is_some()
            && self.res.uses.get(&i.span.start).copied() == round.name
        {
            return;
        }
        // `total += n` on a whole number: the same sum in any order.
        let recorded = (self.expr_types.iter().rev()).find(|(sp, _)| *sp == target.span);
        // A whole number, or a literal still to settle as one (`let total = 0`).
        let whole = recorded.is_some_and(|(_, t)| match self.inf.resolve(t) {
            Ty::Prim(p) => p.is_int(),
            Ty::Var(v) => self.inf.kind(v) == crate::infer::VarKind::IntLiteral,
            _ => false,
        });
        // `total = total + n` and `total = (total + n) % m`: a sum, or a
        // sum kept modulo a number, which native code spreads only where
        // it proves the order can't change it.
        if op == AssignOp::Set && whole && is_sum_of(target, value) {
            return;
        }
        if op == AssignOp::Add && matches!(target.kind, ExprKind::Name(_)) {
            if whole {
                return;
            }
            self.error(target.span, format!("`{name} +=` in an `on cores` loop adds in a different order on each run, which changes a float's last bits: add whole numbers, or collect each round's value in its own slot and add them after (spec 4.9)"));
            return;
        }
        self.error(target.span, format!("this changes `{name}`, which every round shares: a round may write only its own slot of a list (`out[i]`, with the loop's name) or add whole numbers into a `let` (`total += n`, `total = (total + n) % m`) (spec 4.9)"));
    }

    /// `xs.push(…)` and the other changing methods in a round.
    pub(crate) fn check_round_change(&mut self, base: &Expr, method: &str) {
        if let Some((_, name)) = self.outside_round(base) {
            self.error(base.span, format!("`{method}` changes `{name}`, which every round shares: write each round's value into its own slot instead (spec 4.9)"));
        }
    }

    /// `what` does I/O: not in an `on cores` round, whose order would show.
    pub(crate) fn check_round_effect(&mut self, what: &str, span: Span) {
        if self.rounds.last().is_some_and(|r| !r.threads) {
            self.error(span, format!("{what} in an `on cores` loop would happen in a different order on each run: collect what to show in each round's slot, then show it after the loop, or use `on threads` for waiting work (spec 4.9)"));
        }
    }
}

/// `value` is `target + …` or `(target + …) % m`, with `m` a whole number
/// above 0 written in the code.
fn is_sum_of(target: &Expr, value: &Expr) -> bool {
    let ExprKind::Name(t) = &target.kind else {
        return false;
    };
    let sum = match &value.kind {
        ExprKind::Binary(BinaryOp::Rem, sum, m) if matches!(m.kind, ExprKind::Int(n) if n > 0) => {
            sum
        }
        _ => value,
    };
    matches!(&sum.kind, ExprKind::Binary(BinaryOp::Add, left, _)
        if matches!(&left.kind, ExprKind::Name(l) if l.name == t.name))
}
