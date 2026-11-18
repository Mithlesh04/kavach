//! `on cores` and `on threads` before a `for` (spec 4.9): where a loop's
//! rounds run. An `on cores` loop must give the one-core result, so each
//! round may write only its own slot (`out[i]`) or row (`grid[i * w + j]`
//! in `for j in 0..w`) or add whole numbers into an outside `let`, and
//! may do no I/O; `on threads` allows I/O.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};

/// A loop with a place, while its body is checked.
#[derive(Clone)]
pub(crate) struct Round {
    /// The loop's body: names declared before it are outside the round.
    pub(crate) body: Span,
    /// The loop's own name (`i` in `for i in …`), for `out[i]`.
    pub(crate) name: Option<DefId>,
    /// `on threads`: I/O is allowed.
    pub(crate) threads: bool,
    /// The `for j in 0..w` loops inside an `on cores` round, with a `w`
    /// that can't change: each its name and `w`.
    pub(crate) rows: Vec<(DefId, Width)>,
    /// The lists the round writes, each with its row width (`None`: its
    /// own slot), so a list written both ways is refused.
    pub(crate) written: Vec<(DefId, Option<Width>)>,
}

/// A row's width: a whole number written in the code, or a `const` or
/// parameter.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Width {
    Int(u64),
    Name(DefId),
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
            rows: Vec::new(),
            written: Vec::new(),
        });
        f(self);
        self.rounds.pop();
    }

    /// A plain `for` inside an `on cores` round: while `f` checks its
    /// body, `for j in 0..w` with a `w` that can't change gives rows.
    pub(crate) fn in_row(&mut self, pattern: &Pattern, iter: &Expr, f: impl FnOnce(&mut Self)) {
        let row = match (&pattern.kind, &iter.kind) {
            (PatternKind::Binding(j), ExprKind::Binary(BinaryOp::Range, lo, w))
                if matches!(lo.kind, ExprKind::Int(0)) =>
            {
                self.decl.get(&j.span.start).copied().zip(self.width(w))
            }
            _ => None,
        };
        let round = self.rounds.last_mut().filter(|r| !r.threads);
        let Some((row, round)) = row.zip(round) else {
            return f(self);
        };
        round.rows.push(row);
        f(self);
        if let Some(r) = self.rounds.last_mut() {
            r.rows.pop();
        }
    }

    /// `w` as a row's width: a whole number written in the code, or a
    /// name that can't change.
    fn width(&self, w: &Expr) -> Option<Width> {
        match &w.kind {
            ExprKind::Int(n) => Some(Width::Int(*n)),
            ExprKind::Name(id) => {
                let def = *self.res.uses.get(&id.span.start)?;
                matches!(
                    self.res.defs[def].kind,
                    DefKind::Param | DefKind::Local { mutable: false }
                )
                .then_some(Width::Name(def))
            }
            _ => None,
        }
    }

    /// `grid[i * w + j]` in a round: its width when `index` is the round's
    /// name times a row's width plus that row's name, in any order.
    fn row_write(&self, round: &Round, index: &Expr) -> Option<Width> {
        let ExprKind::Binary(BinaryOp::Add, a, b) = &index.kind else {
            return None;
        };
        let name = |e: &Expr| match &e.kind {
            ExprKind::Name(id) => self.res.uses.get(&id.span.start).copied(),
            _ => None,
        };
        [(a, b), (b, a)].into_iter().find_map(|(product, j)| {
            let ExprKind::Binary(BinaryOp::Mul, x, y) = &product.kind else {
                return None;
            };
            let j = name(j)?;
            let (_, width) = round.rows.iter().find(|(row, _)| *row == j)?;
            let fits = |i: &Expr, w: &Expr| name(i) == round.name && self.width(w) == Some(*width);
            (fits(x, y) || fits(y, x)).then_some(*width)
        })
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
        // `out[i] = …`: the round's own slot; `grid[i * w + j]`: its row.
        if let ExprKind::Index { base, index } = &target.kind
            && let ExprKind::Name(list) = &base.kind
            && round.name.is_some()
        {
            let slot = matches!(&index.kind, ExprKind::Name(i)
                if self.res.uses.get(&i.span.start).copied() == round.name);
            let row = self.row_write(&round, index);
            if slot || row.is_some() {
                return self.written_as(list, row);
            }
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
        self.error(target.span, format!("this changes `{name}`, which every round shares: a round may write only its own slot of a list (`out[i]`, with the loop's name), its own row (`grid[i * w + j]` in `for j in 0..w`), or add whole numbers into a `let` (`total += n`, `total = (total + n) % m`) (spec 4.9)"));
    }

    /// Notes that the round writes `list` as its slot (`row` `None`) or
    /// row: a list written two ways is refused.
    fn written_as(&mut self, list: &Ident, row: Option<Width>) {
        let Some(def) = self.res.uses.get(&list.span.start).copied() else {
            return;
        };
        let Some(round) = self.rounds.last_mut() else {
            return;
        };
        match round.written.iter().find(|(d, _)| *d == def) {
            Some((_, before)) if *before != row => {
                let name = &list.name;
                self.error(list.span, format!("`{name}` is written two ways in an `on cores` loop, so two rounds could reach the same item: write each round's own slot (`{name}[i]`) or its own row (`{name}[i * w + j]`), one of them, with one `w` (spec 4.9)"));
            }
            Some(_) => {}
            None => round.written.push((def, row)),
        }
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
