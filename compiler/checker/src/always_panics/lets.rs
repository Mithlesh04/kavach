//! A `let` whose value is known where it is read (spec 1.8): set to a known
//! value, with nothing between that could have changed it another way. A
//! branch, a loop or a lambda may or may not run, so what one sets is known
//! no more, and what a condition tests isn't assumed inside or after it: it
//! may be there to keep the arithmetic safe.

use super::{Holds, Known};
use kavach_ast::visit::{VisitMut, walk_expr, walk_stmt};
use kavach_ast::*;
use kavach_resolver::DefId;
use num_bigint::{BigInt, Sign};
use std::collections::HashSet;

/// The locals assigned in what it visits, and those it names.
struct Mentions<'k, 'c, 'r> {
    known: &'k Known<'c, 'r>,
    set: HashSet<DefId>,
    named: HashSet<DefId>,
}

impl Mentions<'_, '_, '_> {
    fn def(&self, id: &Ident) -> Option<DefId> {
        self.known.checker.res.uses.get(&id.span.start).copied()
    }
}

impl VisitMut for Mentions<'_, '_, '_> {
    fn stmt(&mut self, s: &mut Stmt) {
        if let StmtKind::Assign { target, .. } = &s.kind
            && let ExprKind::Name(id) = &target.kind
            && let Some(d) = self.def(id)
        {
            self.set.insert(d);
        }
        walk_stmt(self, s);
    }

    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Name(id) = &e.kind
            && let Some(d) = self.def(id)
        {
            self.named.insert(d);
        }
        walk_expr(self, e);
    }
}

/// Code that may not run, or may run many times.
pub(super) enum MayNotRun<'n> {
    Stmt(&'n Stmt),
    Expr(&'n Expr),
}

/// The locals assigned anywhere in `b`.
pub(super) fn mentions_set(k: &Known<'_, '_>, b: &Block) -> HashSet<DefId> {
    let mut m = Mentions {
        known: k,
        set: HashSet::new(),
        named: HashSet::new(),
    };
    for s in &mut b.clone().stmts {
        m.stmt(s);
    }
    m.set
}

/// The locals `e` names.
pub(super) fn named_in(k: &Known<'_, '_>, e: &Expr) -> HashSet<DefId> {
    let mut m = Mentions {
        known: k,
        set: HashSet::new(),
        named: HashSet::new(),
    };
    m.expr(&mut e.clone());
    m.named
}

impl Known<'_, '_> {
    /// Before code that may not run: what it assigns is known neither in
    /// it nor after it, nor is what `test` names. Gives what to hand
    /// `leave` after it.
    pub(super) fn enter(&mut self, node: MayNotRun<'_>, test: Option<&Expr>) -> HashSet<DefId> {
        let before = self.frozen.clone();
        if self.lets.is_empty() {
            return before;
        }
        let mut m = Mentions {
            known: &*self,
            set: HashSet::new(),
            named: HashSet::new(),
        };
        match node {
            MayNotRun::Stmt(s) => m.stmt(&mut s.clone()),
            MayNotRun::Expr(e) => walk_expr(&mut m, &mut e.clone()),
        }
        let set = std::mem::take(&mut m.set);
        m.named.clear();
        if let Some(test) = test {
            m.expr(&mut test.clone());
        }
        let named = m.named;
        for d in set.iter().chain(&named) {
            self.lets.remove(d);
        }
        self.frozen.extend(set);
        before
    }

    /// `x = v` or `x op= v`, on a local: what the run would panic on, and
    /// what `x` is now.
    pub(super) fn assign(&mut self, s: &Stmt, target: &Expr, op: AssignOp, value: &Expr) {
        let ExprKind::Name(id) = &target.kind else {
            return;
        };
        let Some(&d) = self.checker.res.uses.get(&id.span.start) else {
            return;
        };
        let new = match op.binary() {
            None => self.holds(value),
            Some(bin) => self.compound(s, target, bin, value).map(Holds::Whole),
        };
        match new {
            Some(v) if !self.frozen.contains(&d) => {
                self.lets.insert(d, v);
            }
            _ => {
                self.lets.remove(&d);
            }
        }
    }

    /// `x op= v`: its value when it is known and fits.
    fn compound(&mut self, s: &Stmt, x: &Expr, op: BinaryOp, value: &Expr) -> Option<BigInt> {
        let b = self.value(value);
        if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
            if b.as_ref().is_some_and(|b| b.sign() == Sign::NoSign) && self.whole(x).is_some() {
                self.errors
                    .push((s.span, super::DIVIDE_BY_ZERO.to_string()));
            }
            return None;
        }
        let (a, b) = (self.value(x)?, b?);
        let p = self.whole(x)?;
        self.arithmetic(s.span, p, op, a, b)
    }

    /// Whether a call of `callee` may change a variable it is given: an
    /// `inout` parameter is written as a plain argument (spec 3.9). A
    /// function called by name says so in its declaration.
    fn may_change_arguments(&self, callee: &Expr) -> bool {
        let ExprKind::Name(f) = &callee.kind else {
            return true;
        };
        let Some(d) = self.checker.res.uses.get(&f.span.start) else {
            return true;
        };
        self.inout_fns.contains(d)
    }

    /// Forgets the locals passed to a call that may change them.
    pub(super) fn after_call(&mut self, callee: &Expr, args: &[Arg]) {
        if self.lets.is_empty() || !self.may_change_arguments(callee) {
            return;
        }
        for a in args {
            if let ExprKind::Name(id) = &a.value.kind
                && let Some(d) = self.checker.res.uses.get(&id.span.start)
            {
                self.lets.remove(d);
            }
        }
    }
}
