//! A `while` loop that never ends (spec 4.7): its condition reads only
//! variables (no calls, which could see the world change), the loop
//! changes none of them, and nothing in it leaves it. `while true` says
//! an endless loop is meant. Also a `for` over `0..=xs.len()` that
//! reads `xs[i]`, whose last turn reads one past the end (spec 4.4).

use super::NeverRuns;
use kavach_ast::visit::{VisitMut, walk_block, walk_expr, walk_stmt};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashSet;

impl NeverRuns<'_> {
    pub(super) fn endless(&mut self, cond: &Expr, body: &Block) {
        let mut reads = Reads {
            names: Vec::new(),
            calls: false,
        };
        reads.expr(&mut cond.clone());
        if reads.calls || reads.names.is_empty() {
            return;
        }
        let mut vars = HashSet::new();
        let mut names = Vec::new();
        for id in &reads.names {
            let Some(&d) = self.res.uses.get(&id.span.start) else {
                return;
            };
            if !matches!(
                self.res.defs[d].kind,
                DefKind::Local { .. } | DefKind::Param
            ) {
                return;
            }
            if vars.insert(d) {
                names.push(format!("`{}`", id.name));
            }
        }
        let mut inside = Inside {
            nr: self,
            vars: &vars,
            changes: false,
            leaves: false,
        };
        walk_block(&mut inside, &mut body.clone());
        if inside.changes || inside.leaves {
            return;
        }
        let (them, change, it) = match names.len() {
            1 => (names[0].clone(), "doesn't change", "it"),
            _ => (names.join(" and "), "don't change", "them"),
        };
        self.found.push((
            cond.span,
            format!(
                "this loop never ends: {them} {change} in it, and nothing in it leaves it. Change {it} in the loop, add a `break`, or write `while true` if it should run forever (spec 4.7)"
            ),
        ));
    }
}

impl NeverRuns<'_> {
    /// `for i in a..=xs.len()` reading `xs[i]`: the last turn reads one
    /// past the end, unless the loop changes `xs` or leaves first.
    pub(super) fn past_end(&mut self, pattern: &Pattern, iter: &Expr, body: &Block) {
        let PatternKind::Binding(i) = &pattern.kind else {
            return;
        };
        let ExprKind::Binary(BinaryOp::RangeInclusive, _, end) = &iter.kind else {
            return;
        };
        let ExprKind::Call { callee, args, .. } = &end.kind else {
            return;
        };
        let ExprKind::Field { base, name, .. } = &callee.kind else {
            return;
        };
        let ExprKind::Name(list) = &base.kind else {
            return;
        };
        let Some(&xs) = self.res.uses.get(&list.span.start) else {
            return;
        };
        if name.name != "len" || !args.is_empty() {
            return;
        }
        let mut reads = ReadsAt {
            res: self.res,
            list: xs,
            at: i.span,
            found: false,
        };
        walk_block(&mut reads, &mut body.clone());
        let vars = HashSet::from([xs]);
        let mut inside = Inside {
            nr: self,
            vars: &vars,
            changes: false,
            leaves: false,
        };
        walk_block(&mut inside, &mut body.clone());
        if !reads.found || inside.changes || inside.leaves {
            return;
        }
        let (i, xs) = (&i.name, &list.name);
        self.found.push((
            iter.span,
            format!(
                "`{i}` reaches `{xs}.len()`, one past the last index of `{xs}`, so `{xs}[{i}]` panics on the last turn: write `..` instead of `..=`, or loop over `{xs}` itself (spec 4.4)"
            ),
        ));
    }
}

/// Whether a loop's body reads `list[i]`, `i` being the variable
/// declared at `at`.
struct ReadsAt<'r> {
    res: &'r kavach_resolver::Resolution,
    list: DefId,
    at: Span,
    found: bool,
}

impl VisitMut for ReadsAt<'_> {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Index { base, index } = &e.kind {
            let def = |x: &Expr| match &x.kind {
                ExprKind::Name(id) => self.res.uses.get(&id.span.start).copied(),
                _ => None,
            };
            let at = def(index).and_then(|d| self.res.defs[d].span);
            self.found |= def(base) == Some(self.list) && at == Some(self.at);
        }
        walk_expr(self, e);
    }
}

/// The names a condition reads, and whether it calls anything.
struct Reads {
    names: Vec<Ident>,
    calls: bool,
}

impl VisitMut for Reads {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Name(id) => self.names.push(id.clone()),
            ExprKind::Call { .. } | ExprKind::Lambda { .. } | ExprKind::DotLambda(_) => {
                self.calls = true;
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}

/// Whether a loop's body changes one of `vars`, or leaves the loop.
struct Inside<'a, 'r> {
    nr: &'a NeverRuns<'r>,
    vars: &'a HashSet<DefId>,
    changes: bool,
    leaves: bool,
}

impl Inside<'_, '_> {
    /// Whether `e` is one of the variables, or a field or item of one.
    fn names_one(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Name(id) => self
                .nr
                .res
                .uses
                .get(&id.span.start)
                .is_some_and(|d| self.vars.contains(d)),
            ExprKind::Field { base, .. }
            | ExprKind::TupleField { base, .. }
            | ExprKind::Index { base, .. } => self.names_one(base),
            _ => false,
        }
    }
}

impl VisitMut for Inside<'_, '_> {
    fn stmt(&mut self, s: &mut Stmt) {
        if let StmtKind::Assign { target, .. } = &s.kind {
            self.changes |= self.names_one(target);
        }
        walk_stmt(self, s);
    }

    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Break(_) | ExprKind::Return(_) | ExprKind::Fail(_) | ExprKind::Try { .. } => {
                self.leaves = true
            }
            ExprKind::Call { callee, args, .. } => {
                // A method of the variable may change it (`xs.push(1)`),
                // and so may a function with an `inout` parameter.
                if let ExprKind::Field { base, .. } = &callee.kind {
                    self.changes |= self.names_one(base);
                }
                let name = match &callee.kind {
                    ExprKind::Name(id) => Some(&id.name),
                    ExprKind::Field { name, .. } => Some(&name.name),
                    _ => None,
                };
                if name.is_some_and(|n| self.nr.inout.contains(n)) {
                    self.changes |= args.iter().any(|a| self.names_one(&a.value));
                }
                if name.is_some_and(|n| n == "panic" || n == "exit") {
                    self.leaves = true;
                }
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}
