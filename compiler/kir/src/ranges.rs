//! Value ranges of integer locals, and the checks they make unnecessary.
//! Spec 5.6 still holds: a check is removed only when it can never fail.
//!
//! For every integer local the pass finds a range `lo..=hi` that holds
//! every value the local can ever have, from every place it is set (the
//! order of statements is ignored, which keeps it simple and safe). Loops
//! make this a fixed point: a range that keeps growing is widened to its
//! type's limits, then narrowed by computing it again from the widened
//! ranges, which stays safe because each step only uses safe ranges.
//!
//! What the ranges prove:
//! - `a + b`, `a - b`, `a * b` that cannot overflow need no check;
//! - `a % c` (a constant `c > 0`) is `a` itself when `0 <= a < c`, and one
//!   compare and subtract when `0 <= a < 2c`.
//!
//! So `total = (total + x % 13) % 1_000_000_007` keeps `total` between 0
//! and the modulus with no division at all: C++ compilers don't track
//! ranges across loop rounds and divide every time.

use crate::ir::*;

mod accumulate;
mod interval;
use interval::{Range, branch_facts, limits};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Growing ranges to a fixed point, widening the ones that keep growing.
    Grow,
    /// Computing each range again from the current ones.
    Narrow,
    /// Ranges are final: rewrite what they prove.
    Rewrite,
}

struct Pass {
    types: Vec<Type>,
    /// `None` until the local is first set.
    ranges: Vec<Option<Range>>,
    /// The ranges being built in a narrowing round.
    next: Vec<Option<Range>>,
    grown: Vec<u8>,
    phase: Phase,
    changed: bool,
    /// What enclosing `if` conditions say about locals (see `branch_facts`).
    refined: Vec<(Local, Range)>,
    /// The ranges accumulators of counted loops stay in (`accumulate.rs`),
    /// which every later range of them is cut to.
    caps: Vec<Option<Range>>,
    /// The ranges of each counted loop's start and end, by its variable,
    /// as the loop sees them (after the `if`s and `Check`s around it).
    bounds: Vec<Option<(Range, Range)>>,
}

/// Finds the ranges of `f`'s integer locals and removes the checks they
/// make unnecessary.
pub fn prove_ranges(f: &mut Function) {
    let n = f.locals.len();
    let mut p = Pass {
        types: f.locals.clone(),
        ranges: vec![None; n],
        next: vec![None; n],
        grown: vec![0; n],
        phase: Phase::Grow,
        changed: true,
        refined: Vec::new(),
        caps: vec![None; n],
        bounds: vec![None; n],
    };
    let mut rounds = 0;
    while p.changed {
        rounds += 1;
        if rounds > 100 {
            return; // No fixed point: leave every check in place.
        }
        p.changed = false;
        p.round(f);
    }
    // A loop's accumulator grew to its type's limits; its rounds bound it.
    for (l, cap) in accumulate::caps(f, &p.ranges, &p.bounds) {
        let i = l as usize;
        p.caps[i] = Some(cap);
        p.ranges[i] = Some(p.ranges[i].and_then(|r| r.meet(cap)).unwrap_or(cap));
    }
    p.phase = Phase::Narrow;
    for _ in 0..2 {
        p.next = vec![None; n];
        p.round(f);
        p.ranges = std::mem::take(&mut p.next);
    }
    p.phase = Phase::Rewrite;
    p.round(f);
}

impl Pass {
    fn round(&mut self, f: &mut Function) {
        for &param in &f.params {
            self.set(param, Some(limits(self.types[param as usize])));
        }
        self.block(&mut f.body);
    }

    /// Local `l` may hold a value in `r`.
    fn set(&mut self, l: Local, r: Option<Range>) {
        let t = self.types[l as usize];
        let (Some(r), true) = (r, t.is_int()) else {
            return;
        };
        let r = r.clamp(t);
        let i = l as usize;
        let r = match self.caps[i] {
            Some(cap) => r.meet(cap).unwrap_or(cap),
            None => r,
        };
        match self.phase {
            Phase::Grow => {
                let Some(old) = self.ranges[i] else {
                    self.ranges[i] = Some(r);
                    self.changed = true;
                    return;
                };
                let mut new = old.join(r);
                if new == old {
                    return;
                }
                self.grown[i] += 1;
                if self.grown[i] > 2 {
                    let l = limits(t);
                    if new.lo < old.lo {
                        new.lo = l.lo;
                    }
                    if new.hi > old.hi {
                        new.hi = l.hi;
                    }
                }
                self.ranges[i] = Some(new);
                self.changed = true;
            }
            Phase::Narrow => self.next[i] = Some(self.next[i].map_or(r, |o| o.join(r))),
            Phase::Rewrite => {}
        }
    }

    fn block(&mut self, b: &mut Block) -> Option<Range> {
        // Past a `Check`, its condition holds: for the rest of the block,
        // when the block never sets the local it is about.
        let outer = self.refined.len();
        for i in 0..b.stmts.len() {
            self.stmt(&mut b.stmts[i]);
            let fact = match &b.stmts[i] {
                Stmt::Check { cond, .. } => branch_facts(cond),
                _ => None,
            };
            if let Some((l, yes, _)) = fact.filter(|(l, ..)| !crate::walk::sets_local(b, *l)) {
                self.refined.push((l, yes));
            }
        }
        let r = b.value.as_mut().and_then(|v| self.expr(v));
        self.refined.truncate(outer);
        r
    }

    fn stmt(&mut self, s: &mut Stmt) {
        match s {
            Stmt::Let(l, e) => {
                let r = self.expr(e);
                self.set(*l, r);
            }
            Stmt::Expr(e) | Stmt::Return(Some(e)) => {
                self.expr(e);
            }
            Stmt::While { cond, body } => {
                self.expr(cond);
                self.block(body);
            }
            Stmt::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
            } => {
                let (s, e) = (self.expr(start), self.expr(end));
                if let (Some(s), Some(e), Phase::Grow) = (s, e, self.phase) {
                    let seen = &mut self.bounds[*var as usize];
                    *seen = Some(seen.map_or((s, e), |(a, b)| (a.join(s), b.join(e))));
                }
                if let (Some(s), Some(e)) = (s, e) {
                    let hi = if *inclusive { e.hi } else { e.hi - 1 };
                    if s.lo <= hi {
                        self.set(*var, Some(Range { lo: s.lo, hi }));
                    }
                }
                self.block(body);
            }
            Stmt::ForEach {
                var, index, body, ..
            } => {
                self.set(*var, Some(limits(self.types[*var as usize])));
                self.set(
                    *index,
                    Some(Range {
                        lo: 0,
                        hi: i64::MAX as i128,
                    }),
                );
                self.block(body);
            }
            Stmt::LetResult { value, call, .. } => {
                self.expr(call);
                self.set(*value, Some(limits(self.types[*value as usize])));
            }
            Stmt::Print(parts) => {
                for part in parts {
                    if let PrintPart::Value(e) = part {
                        self.expr(e);
                    }
                }
            }
            Stmt::Check { cond, .. } => {
                self.expr(cond);
            }
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }

    /// The range of `e` (`None` when it has no value yet, or is not an
    /// integer). Visits every part, recording where locals are set.
    fn expr(&mut self, e: &mut Expr) -> Option<Range> {
        let ty = e.ty;
        let r = match &mut e.kind {
            ExprKind::Int(n) => Some(Range { lo: *n, hi: *n }),
            ExprKind::Local(l) | ExprKind::Move(l) => self.read(*l),
            ExprKind::Assign(l, value) => {
                let r = self.expr(value);
                self.set(*l, r);
                None
            }
            ExprKind::Binary(op, a, b) => {
                let (op, ra, rb) = (*op, self.expr(a), self.expr(b));
                return self.binary(e, op, ra, rb);
            }
            ExprKind::Unary(UnOp::Neg, x) => self.expr(x).map(|r| Range {
                lo: -r.hi,
                hi: -r.lo,
            }),
            ExprKind::Convert(x, checked) => {
                let r = self.expr(x);
                if !x.ty.is_int() {
                    Some(limits(ty))
                } else {
                    // A value proven to fit needs no run-time check.
                    let fits = |r: &Range| {
                        let l = limits(ty);
                        l.lo <= r.lo && r.hi <= l.hi
                    };
                    if ty.is_int() && r.as_ref().is_some_and(fits) {
                        *checked = false;
                    }
                    r
                }
            }
            ExprKind::If(cond, then, otherwise) => {
                self.expr(cond);
                let facts = branch_facts(cond);
                let a = self.refined_block(then, facts.map(|(l, yes, _)| (l, yes)));
                let b = self.refined_block(otherwise, facts.map(|(l, _, no)| (l, no)));
                match (a, b) {
                    (Some(a), Some(b)) => Some(a.join(b)),
                    (a, b) => a.or(b),
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fallback(fast, slow) => match (self.block(fast), self.block(slow)) {
                (Some(a), Some(b)) => Some(a.join(b)),
                (a, b) => a.or(b),
            },
            ExprKind::Rt(crate::RtOp::ListLen | crate::RtOp::StrLen, args) => {
                self.all(args);
                Some(Range {
                    lo: 0,
                    hi: i64::MAX as i128,
                })
            }
            _ => {
                self.parts(e);
                Some(limits(ty))
            }
        };
        if ty.is_int() {
            r.map(|r| r.clamp(ty))
        } else {
            None
        }
    }

    /// The range of local `l` where it is read: narrowed by the conditions
    /// of enclosing `if`s (`None` where the branch can't run).
    fn read(&self, l: Local) -> Option<Range> {
        self.refined
            .iter()
            .filter(|(x, _)| *x == l)
            .try_fold(self.ranges[l as usize]?, |r, (_, fact)| r.meet(*fact))
    }

    /// The range of block `b`, a branch where `fact` holds, unless the
    /// branch itself sets the local.
    fn refined_block(&mut self, b: &mut Block, fact: Option<(Local, Range)>) -> Option<Range> {
        let fact = fact.filter(|(l, _)| !crate::walk::sets_local(b, *l));
        if let Some(f) = fact {
            self.refined.push(f);
        }
        let r = self.block(b);
        if fact.is_some() {
            self.refined.pop();
        }
        r
    }

    fn all(&mut self, items: &mut [Expr]) {
        for x in items {
            self.expr(x);
        }
    }

    /// Visits the parts of an expression whose range isn't tracked.
    fn parts(&mut self, e: &mut Expr) {
        match &mut e.kind {
            ExprKind::Unary(_, x)
            | ExprKind::Convert(x, _)
            | ExprKind::Some(x)
            | ExprKind::IsSome(x)
            | ExprKind::Unwrap(x)
            | ExprKind::GetField(x, _)
            | ExprKind::HasVariant(x, _)
            | ExprKind::Parse(x)
            | ExprKind::Fail(x) => {
                self.expr(x);
            }
            ExprKind::Fast(_, a, b)
            | ExprKind::Index(a, b)
            | ExprKind::InRange(a, b)
            | ExprKind::Context(a, b)
            | ExprKind::SetField(a, _, b)
            | ExprKind::And(a, b)
            | ExprKind::Or(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Format(items)
            | ExprKind::List(items)
            | ExprKind::Tuple(items)
            | ExprKind::Call(_, items)
            | ExprKind::Rt(_, items)
            | ExprKind::Record { fields: items, .. }
            | ExprKind::Closure {
                captures: items, ..
            } => self.all(items),
            ExprKind::CallValue(callee, args) => {
                self.expr(callee);
                self.all(args);
            }
            ExprKind::Map(pairs) => {
                for (k, v) in pairs {
                    self.expr(k);
                    self.expr(v);
                }
            }
            _ => {}
        }
    }

    /// The range of `a op b`; in the last round, rewrites `e` when the
    /// ranges prove a check unnecessary.
    fn binary(
        &mut self,
        e: &mut Expr,
        op: BinOp,
        ra: Option<Range>,
        rb: Option<Range>,
    ) -> Option<Range> {
        let ty = e.ty;
        if !ty.is_int() {
            return None;
        }
        let (Some(a), Some(b)) = (ra, rb) else {
            return Some(limits(ty));
        };
        let corners = |f: fn(i128, i128) -> i128| {
            let c = [f(a.lo, b.lo), f(a.lo, b.hi), f(a.hi, b.lo), f(a.hi, b.hi)];
            Range {
                lo: *c.iter().min().expect("four corners"),
                hi: *c.iter().max().expect("four corners"),
            }
        };
        let (r, fast) = match op {
            BinOp::Add => (corners(i128::saturating_add), FastOp::Add),
            BinOp::Sub => (corners(i128::saturating_sub), FastOp::Sub),
            BinOp::Mul => (corners(i128::saturating_mul), FastOp::Mul),
            BinOp::Rem if b.lo == b.hi && b.lo > 0 => {
                let c = b.lo;
                let r = if a.lo >= 0 {
                    Range {
                        lo: 0,
                        hi: a.hi.min(c - 1),
                    }
                } else {
                    Range {
                        lo: a.lo.max(-(c - 1)),
                        hi: a.hi.min(c - 1),
                    }
                };
                if self.phase == Phase::Rewrite && a.lo >= 0 {
                    rewrite_rem(e, a, c);
                }
                return Some(r);
            }
            BinOp::Div if b.lo == b.hi && b.lo > 0 => {
                if self.phase == Phase::Rewrite && a.lo >= 0 {
                    let small = a.hi.max(b.hi) <= SMALL;
                    to_fast(
                        e,
                        if small {
                            FastOp::DivSmall
                        } else {
                            FastOp::DivPositive
                        },
                    );
                }
                return Some(Range {
                    lo: a.lo / b.lo,
                    hi: a.hi / b.lo,
                });
            }
            BinOp::BitAnd if b.lo >= 0 && a.lo >= 0 => {
                return Some(Range {
                    lo: 0,
                    hi: a.hi.min(b.hi),
                });
            }
            // A mask that isn't negative clears the sign: `t & 1` is 0 or
            // 1 whatever `t` is.
            BinOp::BitAnd if b.lo >= 0 => return Some(Range { lo: 0, hi: b.hi }),
            _ => return Some(limits(ty)),
        };
        if self.phase == Phase::Rewrite && r.within(ty) {
            to_fast(e, fast);
        }
        Some(r.clamp(ty))
    }
}

/// The largest value 32-bit division takes.
const SMALL: i128 = u32::MAX as i128;

/// Turns the binary expression `e` into the proven operation `op`.
fn to_fast(e: &mut Expr, op: FastOp) {
    let ExprKind::Binary(_, x, y) = std::mem::replace(&mut e.kind, ExprKind::None) else {
        unreachable!("a binary expression");
    };
    e.kind = ExprKind::Fast(op, x, y);
}

/// `a % c` with `a >= 0`: `a` itself when it is always below `c`, one
/// subtract when it is below `2c`, otherwise an unsigned remainder.
fn rewrite_rem(e: &mut Expr, a: Range, c: i128) {
    if a.hi >= c {
        let op = if a.hi < 2 * c {
            FastOp::RemOnce
        } else if a.hi.max(c) <= SMALL {
            FastOp::RemSmall
        } else {
            FastOp::RemPositive
        };
        return to_fast(e, op);
    }
    if let ExprKind::Binary(_, x, _) = std::mem::replace(&mut e.kind, ExprKind::None) {
        e.kind = x.kind;
    }
}

#[cfg(test)]
mod tests;
