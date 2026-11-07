//! Moving values at their last use (spec 6.1). A heap value read from a
//! variable is normally shared: the variable keeps it, so a later change
//! must copy it. When the variable is never read again, the read becomes
//! an [`ExprKind::Move`]: the value is handed over and the variable is
//! emptied, so the new owner can change it in place. Passing a list to a
//! function, returning it, or `var copy = items` then costs no copy.
//!
//! This is a backward liveness pass over each function. It must follow
//! the order in which the code generator evaluates operands.
//!
//! A variable that holds a `Drop` value is never moved here: it keeps the
//! value to the end of its block (`drop_scopes.rs` ends it there), as the
//! interpreter does, so a function it is passed to, or an `inout`
//! parameter given a new value, never drops it first (spec 6.1, 3.9).

use crate::ir::{Block, Expr, ExprKind, Function, Local, PrintPart, Stmt, Type};
use std::collections::{BTreeSet, HashSet};

type Live = BTreeSet<Local>;

struct Pass<'f> {
    types: &'f [Type],
    /// The types that hold `Drop` values, which are never moved.
    holders: &'f HashSet<Type>,
    /// Whether to rewrite last reads; off while a loop's live set is
    /// still being worked out.
    mark: bool,
    /// For each enclosing loop: (live at its start, live after it).
    loops: Vec<(Live, Live)>,
}

/// Rewrites the last read of each heap variable in `f` into a move,
/// except for variables of the `holders` types.
pub fn mark_moves(f: &mut Function, holders: &HashSet<Type>) {
    let types = f.locals.clone();
    let mut pass = Pass {
        types: &types,
        holders,
        mark: true,
        loops: Vec::new(),
    };
    pass.block(&mut f.body, Live::new());
}

impl Pass<'_> {
    fn block(&mut self, b: &mut Block, mut live: Live) -> Live {
        if let Some(v) = &mut b.value {
            live = self.expr(v, live);
        }
        for s in b.stmts.iter_mut().rev() {
            live = self.stmt(s, live);
        }
        live
    }

    /// The variables live at the start of a loop. Each round evaluates
    /// `cond` (if any), defines `kills`, reads `uses` and runs `body`.
    fn loop_start(
        &mut self,
        mut cond: Option<&mut Expr>,
        body: &mut Block,
        after: &Live,
        kills: &[Local],
        uses: &[Local],
    ) -> Live {
        let mark = std::mem::replace(&mut self.mark, false);
        let mut start = Live::new();
        loop {
            let next = self.round(cond.as_deref_mut(), body, &start, after, kills, uses);
            if next == start {
                break;
            }
            start = next;
        }
        self.mark = mark;
        self.round(cond, body, &start, after, kills, uses);
        start
    }

    fn round(
        &mut self,
        cond: Option<&mut Expr>,
        body: &mut Block,
        start: &Live,
        after: &Live,
        kills: &[Local],
        uses: &[Local],
    ) -> Live {
        self.loops.push((start.clone(), after.clone()));
        let mut body_in = self.block(body, start.clone());
        self.loops.pop();
        for k in kills {
            body_in.remove(k);
        }
        let mut live = after.clone();
        live.extend(body_in);
        live.extend(uses.iter().copied());
        match cond {
            Some(c) => self.expr(c, live),
            None => live,
        }
    }

    fn stmt(&mut self, s: &mut Stmt, mut live: Live) -> Live {
        match s {
            Stmt::Let(l, e) => {
                live.remove(l);
                self.expr(e, live)
            }
            Stmt::Expr(e) | Stmt::Check { cond: e, .. } => self.expr(e, live),
            Stmt::LetResult { value, error, call } => {
                live.remove(value);
                live.remove(error);
                self.expr(call, live)
            }
            Stmt::Return(value) => match value {
                Some(e) => self.expr(e, Live::new()),
                None => Live::new(),
            },
            Stmt::Break => self.loops.last().map(|l| l.1.clone()).unwrap_or_default(),
            Stmt::Continue => self.loops.last().map(|l| l.0.clone()).unwrap_or_default(),
            Stmt::Print(parts) => {
                for p in parts.iter_mut().rev() {
                    if let PrintPart::Value(e) = p {
                        live = self.expr(e, live);
                    }
                }
                live
            }
            Stmt::While { cond, body } => self.loop_start(Some(cond), body, &live, &[], &[]),
            Stmt::ForRange {
                var,
                start,
                end,
                body,
                ..
            } => {
                let live = self.loop_start(None, body, &live, &[*var], &[]);
                let live = self.expr(end, live);
                self.expr(start, live)
            }
            Stmt::ForEach {
                var,
                index,
                list,
                body,
            } => self.loop_start(None, body, &live, &[*var, *index], &[*list]),
        }
    }

    /// A read of local `l`: a move when nothing reads `l` afterwards.
    fn read(&mut self, e: &mut Expr, l: Local, mut live: Live) -> Live {
        let ty = self.types[l as usize];
        if self.mark && ty.is_ref() && !live.contains(&l) && !self.holders.contains(&ty) {
            e.kind = ExprKind::Move(l);
        }
        live.insert(l);
        live
    }

    /// Operands evaluated left to right, so analysed right to left.
    fn all<'e>(
        &mut self,
        items: impl DoubleEndedIterator<Item = &'e mut Expr>,
        mut live: Live,
    ) -> Live {
        for x in items.rev() {
            live = self.expr(x, live);
        }
        live
    }

    fn expr(&mut self, e: &mut Expr, mut live: Live) -> Live {
        match &mut e.kind {
            ExprKind::Local(l) | ExprKind::Move(l) => {
                let l = *l;
                self.read(e, l, live)
            }
            ExprKind::Assign(l, value) => {
                let l = *l;
                live.remove(&l);
                self.assign(l, value, live)
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::Decimal(_)
            | ExprKind::None => live,
            ExprKind::Panic(_) => Live::new(),
            ExprKind::Fail(x) => self.expr(x, Live::new()),
            ExprKind::Unary(_, x)
            | ExprKind::Convert(x, _)
            | ExprKind::Some(x)
            | ExprKind::IsSome(x)
            | ExprKind::Unwrap(x)
            | ExprKind::GetField(x, _)
            | ExprKind::HasVariant(x, _)
            | ExprKind::Parse(x) => self.expr(x, live),
            ExprKind::Binary(_, a, b)
            | ExprKind::Fast(_, a, b)
            | ExprKind::Index(a, b)
            | ExprKind::InRange(a, b)
            | ExprKind::Context(a, b)
            | ExprKind::SetField(a, _, b) => {
                let live = self.expr(b, live);
                self.expr(a, live)
            }
            ExprKind::And(a, b) | ExprKind::Or(a, b) => {
                // `b` may not run.
                let mut both = self.expr(b, live.clone());
                both.extend(live);
                self.expr(a, both)
            }
            ExprKind::If(cond, then, otherwise) => {
                let mut both = self.block(then, live.clone());
                both.extend(self.block(otherwise, live));
                self.expr(cond, both)
            }
            ExprKind::Block(b) => self.block(b, live),
            // `slow` may run after part of `fast`: what it reads stays
            // live through all of `fast`.
            ExprKind::Fallback(fast, slow) => {
                let mut either = self.block(slow, live.clone());
                either.extend(live);
                self.block(fast, either)
            }
            ExprKind::Format(items)
            | ExprKind::List(items)
            | ExprKind::Tuple(items)
            | ExprKind::Call(_, items)
            | ExprKind::Rt(_, items)
            | ExprKind::Record { fields: items, .. }
            | ExprKind::Closure {
                captures: items, ..
            } => self.all(items.iter_mut(), live),
            ExprKind::Map(pairs) => {
                for (k, v) in pairs.iter_mut().rev() {
                    live = self.expr(v, live);
                    live = self.expr(k, live);
                }
                live
            }
            ExprKind::CallValue(callee, args) => {
                let live = self.all(args.iter_mut(), live);
                self.expr(callee, live)
            }
        }
    }

    /// `l = value`. When `value` changes `l`'s own list or record
    /// (`items.push(x)`, `p.x = 1`), the code generator evaluates the
    /// other operands first and reads `l` last (`errors.rs` in the
    /// backend), so the read of `l` is analysed first.
    fn assign(&mut self, l: Local, value: &mut Expr, live: Live) -> Live {
        let is_l = |e: &Expr| matches!(e.kind, ExprKind::Local(x) | ExprKind::Move(x) if x == l);
        match &mut value.kind {
            ExprKind::Rt(op, args) if op.changes_receiver() && is_l(&args[0]) => {
                let (first, rest) = args.split_at_mut(1);
                let live = self.expr(&mut first[0], live);
                self.all(rest.iter_mut(), live)
            }
            ExprKind::SetField(base, _, field) if is_l(base) => {
                let live = self.expr(base, live);
                self.expr(field, live)
            }
            _ => self.expr(value, live),
        }
    }
}
