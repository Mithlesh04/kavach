//! Arithmetic that always panics (spec 1.8): a whole-number division or
//! remainder by a `0` the compiler knows, and `+`, `-` or `*` of values
//! it knows (literals, `const`s set to them, and `let`s where nothing may
//! have changed them, `always_panics/lets.rs`) whose result doesn't fit
//! the type. Each is a compile error, with the message the run would
//! give. Literals alone are checked as constants already (`constants.rs`).
//! So is an index outside a list whose length is known; and `/` between
//! two whole literals that don't divide evenly gets a warning.
//! A call whose known arguments break a `require` always panics too, as
//! does a known result that breaks an `ensure` (spec 7.7,
//! `always_panics/contracts.rs`).

use crate::checker::Checker;
use crate::constants::fits;
use crate::ty::{Prim, Ty};
use kavach_ast::visit::{VisitMut, walk_expr, walk_item, walk_stmt};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use num_bigint::{BigInt, Sign};
use std::collections::{HashMap, HashSet};

const DIVIDE_BY_ZERO: &str =
    "this always panics: division by zero. Divide by something that can't be 0 (spec 1.8)";

impl Checker<'_> {
    pub(crate) fn check_always_panics(&mut self, file: &File) {
        let types = self
            .expr_types
            .iter()
            .map(|(s, t)| ((s.start, s.end), self.defaulted(t)))
            .collect();
        let mut found = Known {
            checker: self,
            types,
            consts: HashMap::new(),
            lets: HashMap::new(),
            frozen: HashSet::new(),
            ranges: HashMap::new(),
            inout_fns: HashSet::new(),
            lists: HashMap::new(),
            contracts: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        found.contracts = contracts::contracts(&found, file);
        found.inout_fns = contracts::inout_fns(&found, file);
        for item in &mut file.clone().items {
            // A function can't rely on what top-level code set.
            if !matches!(item.kind, ItemKind::Stmt(_)) {
                found.lets.clear();
            }
            walk_item(&mut found, item);
        }
        found.lets.clear();
        found.results_against_ensures(file);
        let errors = std::mem::take(&mut found.errors);
        let warnings = std::mem::take(&mut found.warnings);
        for (span, message) in errors {
            self.error(span, message);
        }
        for (span, message) in warnings {
            self.warn(span, &message);
        }
    }
}

struct Known<'c, 'r> {
    checker: &'c Checker<'r>,
    types: HashMap<(u32, u32), Ty>,
    /// The `const`s set to a whole number the compiler knows.
    consts: HashMap<DefId, BigInt>,
    /// The `let`s whose value is known here.
    lets: HashMap<DefId, BigInt>,
    /// The `let`s set in code that may not run, around what is visited.
    frozen: HashSet<DefId>,
    /// The variables of the known loops around what is visited, each with
    /// its first and last value, while what is visited always runs.
    ranges: HashMap<DefId, (BigInt, BigInt)>,
    /// The functions with an `inout` parameter (spec 3.9).
    inout_fns: HashSet<DefId>,
    /// The `const`s set to a list literal, with its length.
    lists: HashMap<DefId, usize>,
    /// What calls of functions with a `require` are checked against.
    contracts: HashMap<DefId, contracts::Contract>,
    errors: Vec<(Span, String)>,
    warnings: Vec<(Span, String)>,
}

mod contracts;
mod lets;
mod rounds;

impl Known<'_, '_> {
    /// The value of `e` when the compiler knows it: whole-number literals
    /// and known `const`s, joined by `+ - *` and a minus, and converted to
    /// a whole-number type they fit (`Int8(i)`).
    fn value(&self, e: &Expr) -> Option<BigInt> {
        match &e.kind {
            ExprKind::Int(n) => Some(BigInt::from(*n)),
            ExprKind::Name(id) => {
                let d = self.checker.res.uses.get(&id.span.start)?;
                self.consts.get(d).or_else(|| self.lets.get(d)).cloned()
            }
            ExprKind::Unary(UnaryOp::Neg, x) => Some(-self.value(x)?),
            ExprKind::Call { callee, args, .. } => self.converted(callee, args),
            ExprKind::Binary(op, l, r) => {
                let (a, b) = (self.value(l)?, self.value(r)?);
                Some(match op {
                    BinaryOp::Add => a + b,
                    BinaryOp::Sub => a - b,
                    BinaryOp::Mul => a * b,
                    _ => return None,
                })
            }
            _ => None,
        }
    }

    /// `Int8(x)` of a known whole number `x` that fits the type: `x`. One
    /// that doesn't fit panics, which this doesn't claim to know.
    fn converted(&self, callee: &Expr, args: &[Arg]) -> Option<BigInt> {
        let (ExprKind::Name(id), [arg]) = (&callee.kind, args) else {
            return None;
        };
        let def = &self.checker.res.defs[*self.checker.res.uses.get(&id.span.start)?];
        if !matches!(def.kind, DefKind::Builtin) || arg.name.is_some() {
            return None;
        }
        let p = Prim::from_name(&def.name).filter(|p| p.is_int())?;
        self.value(&arg.value).filter(|v| fits(v, p))
    }

    /// The whole-number type of `e`, if it has one.
    fn whole(&self, e: &Expr) -> Option<Prim> {
        match self.types.get(&(e.span.start, e.span.end))? {
            Ty::Prim(p) if p.is_int() => Some(*p),
            _ => None,
        }
    }

    fn binary(&mut self, e: &Expr, op: BinaryOp, l: &Expr, r: &Expr) {
        let Some(p) = self.whole(e) else { return };
        if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
            let b = self.value(r);
            if b.as_ref().is_some_and(|b| b.sign() == Sign::NoSign) {
                self.errors.push((e.span, DIVIDE_BY_ZERO.to_string()));
            } else if op == BinaryOp::Div {
                self.uneven(e, l, r);
            }
            return;
        }
        let names = |x: &Expr| crate::constants::const_int(x).is_none();
        if !matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul) || !(names(l) || names(r)) {
            return;
        }
        let (Some(a), Some(b)) = (self.value(l), self.value(r)) else {
            return;
        };
        self.arithmetic(e.span, p, op, a, b);
    }

    /// `a op b` of type `p`, for `+`, `-` or `*`: its value, or an error at
    /// `span` when it doesn't fit.
    fn arithmetic(
        &mut self,
        span: Span,
        p: Prim,
        op: BinaryOp,
        a: BigInt,
        b: BigInt,
    ) -> Option<BigInt> {
        // Only the innermost: an operand that overflows was reported.
        if !fits(&a, p) || !fits(&b, p) {
            return None;
        }
        let (v, sign) = match op {
            BinaryOp::Add => (&a + &b, "+"),
            BinaryOp::Sub => (&a - &b, "-"),
            BinaryOp::Mul => (&a * &b, "*"),
            _ => return None,
        };
        if fits(&v, p) {
            return Some(v);
        }
        self.errors.push((
            span,
            format!(
                "this always panics: integer overflow: {a} {sign} {b} doesn't fit in {} (spec 1.8)",
                p.name()
            ),
        ));
        None
    }
}

impl Known<'_, '_> {
    /// `1 / 2`: two whole literals that don't divide evenly, so the
    /// fraction is lost (spec 1.8).
    fn uneven(&mut self, e: &Expr, l: &Expr, r: &Expr) {
        fn literal(x: &Expr) -> Option<BigInt> {
            match &x.kind {
                ExprKind::Int(n) => Some(BigInt::from(*n)),
                ExprKind::Unary(UnaryOp::Neg, x) => literal(x).map(|n| -n),
                _ => None,
            }
        }
        let (Some(a), Some(b)) = (literal(l), literal(r)) else {
            return;
        };
        if b.sign() == Sign::NoSign || (&a % &b).sign() == Sign::NoSign {
            return;
        }
        // `-7.div_floor(2)` would be `-(7.div_floor(2))`.
        let receiver = match a.sign() {
            Sign::Minus => format!("({a})"),
            _ => a.to_string(),
        };
        let message = format!(
            "`{a} / {b}` is `{}`: whole numbers divide to a whole number. Write `{a}.0 / {b}.0` for a fraction, or `{receiver}.div_floor({b})` when the whole part is meant (spec 1.8)",
            &a / &b
        );
        self.warnings.push((e.span, message));
    }

    /// `xs[i]` with an `i` outside a list whose length is known.
    fn index(&mut self, e: &Expr, base: &Expr, index: &Expr) {
        let len = match &base.kind {
            ExprKind::List(items) => items.len(),
            ExprKind::Name(id) => {
                let Some(d) = self.checker.res.uses.get(&id.span.start) else {
                    return;
                };
                let Some(&len) = self.lists.get(d) else {
                    return;
                };
                len
            }
            _ => return,
        };
        let Some(i) = self.value(index) else { return };
        if i.sign() != Sign::Minus && i < BigInt::from(len) {
            return;
        }
        let items = if len == 1 { "item" } else { "items" };
        self.errors.push((
            e.span,
            format!(
                "this always panics: index {i} is out of range: the list has {len} {items} (spec 1.8)"
            ),
        ));
    }
}

impl VisitMut for Known<'_, '_> {
    fn stmt(&mut self, s: &mut Stmt) {
        let mut loop_var = None;
        let mut ranges_outside = None;
        let frozen = match &s.kind {
            StmtKind::For {
                pattern,
                iter,
                body,
                ..
            } => {
                match self.known_rounds(pattern, iter, body) {
                    Some(rounds) => {
                        self.accumulators(&rounds, body);
                        loop_var = rounds.var.map(|d| (d, (rounds.first, rounds.last)));
                    }
                    None => ranges_outside = Some(std::mem::take(&mut self.ranges)),
                }
                Some(self.enter(lets::MayNotRun::Stmt(s), None))
            }
            StmtKind::While { cond, .. } => {
                ranges_outside = Some(std::mem::take(&mut self.ranges));
                Some(self.enter(lets::MayNotRun::Stmt(s), Some(cond)))
            }
            _ => None,
        };
        let var = loop_var.map(|(d, r)| {
            self.ranges.insert(d, r);
            d
        });
        walk_stmt(self, s);
        if let Some(d) = var {
            self.ranges.remove(&d);
        }
        if let Some(ranges) = ranges_outside {
            self.ranges = ranges;
        }
        if let Some(frozen) = frozen {
            self.frozen = frozen;
        }
        match &s.kind {
            StmtKind::Let {
                mutable,
                pattern,
                value,
                ..
            } => {
                let PatternKind::Binding(id) = &pattern.kind else {
                    return;
                };
                let known = self.value(value);
                let def = self.checker.decl.get(&id.span.start).copied();
                if *mutable {
                    if let (Some(v), Some(d)) = (known, def)
                        && !self.frozen.contains(&d)
                    {
                        self.lets.insert(d, v);
                    }
                    return;
                }
                if let (Some(v), Some(d)) = (known, def) {
                    self.consts.insert(d, v);
                }
                if let (ExprKind::List(items), Some(d)) = (&value.kind, def) {
                    self.lists.insert(d, items.len());
                }
            }
            StmtKind::Assign { target, op, value } => self.assign(s, target, *op, value),
            _ => {}
        }
    }

    fn expr(&mut self, e: &mut Expr) {
        use lets::MayNotRun::Expr as Node;
        let frozen = match &e.kind {
            ExprKind::If { cond, .. } => Some(self.enter(Node(e), Some(cond))),
            ExprKind::IfLet { value, .. } => Some(self.enter(Node(e), Some(value))),
            ExprKind::Match { scrutinee, .. } => Some(self.enter(Node(e), Some(scrutinee))),
            ExprKind::Catch { .. } | ExprKind::Binary(BinaryOp::And | BinaryOp::Or, ..) => {
                Some(self.enter(Node(e), None))
            }
            _ => None,
        };
        // A lambda or a task runs later, if at all, with its own copy.
        let later = matches!(
            e.kind,
            ExprKind::Lambda { .. } | ExprKind::DotLambda(_) | ExprKind::Spawn(..)
        );
        let lets_outside = later.then(|| std::mem::take(&mut self.lets));
        // What may not run isn't checked for a loop's every round.
        let ranges_outside = (later || frozen.is_some()).then(|| std::mem::take(&mut self.ranges));
        // Unsafe code can reach a variable through a pointer.
        if matches!(e.kind, ExprKind::Unsafe(_)) {
            self.lets.clear();
        }
        walk_expr(self, e);
        if let Some(lets) = lets_outside {
            self.lets = lets;
        }
        if let Some(ranges) = ranges_outside {
            self.ranges = ranges;
        }
        if let Some(frozen) = frozen {
            self.frozen = frozen;
        }
        match &e.kind {
            ExprKind::Binary(op, l, r) => self.each_round(|k| k.binary(e, *op, l, r)),
            ExprKind::Call { callee, args, .. } => {
                self.each_round(|k| k.call(e, callee, args));
                self.after_call(callee, args);
            }
            ExprKind::Index { base, index } => self.each_round(|k| k.index(e, base, index)),
            _ => {}
        }
    }
}
