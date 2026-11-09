//! Fusing list pipelines. `let doubled = items.map(f)`, then
//! `let big = doubled.filter(g)`, then `big.sum(h)` builds two lists that
//! nobody keeps. Each method is a loop (`lower_lambda.rs`); when the list
//! one loop builds is used only by the loop in the next statement, the
//! two become one loop: each item the first makes goes straight through
//! the second, and the list in between is never built.
//!
//! The fused loop does the same operations in another order, so a check
//! that fails (an overflow, an index out of range) could fail at another
//! place than in the program as written. So it is the fast block of a
//! `Fallback`: when a check in it fails, the code as written runs
//! instead and fails where the program says. Every operation of the fast
//! block is also one of the code as written, so that code fails too.
//! Only code without effects is fused: arithmetic, comparisons, reading
//! lists and text, and the loops' own locals.

use crate::ir::*;
use crate::walk::{any_expr, any_in_block, exprs_mut, locals_mut};

pub fn fuse_loops(f: &mut Function, shapes: &[Shape]) {
    let log = std::env::var_os("KAVACH_FUSE_SKELETON").is_some() && f.name == "main";
    if log {
        eprintln!("skeleton of {} before:", f.name);
        hoist::skeleton(&f.body, 1);
    }
    let mut reads = vec![0usize; f.locals.len()];
    any_in_block(&f.body, &mut |e| {
        if let ExprKind::Local(l) | ExprKind::Move(l) = e.kind {
            reads[l as usize] += 1;
        }
        false
    });
    let mut cx = Cx {
        locals: std::mem::take(&mut f.locals),
        reads,
        shapes,
        name: &f.name,
    };
    cx.block(&mut f.body);
    f.locals = cx.locals;
    if log {
        eprintln!("skeleton of {} after:", f.name);
        hoist::skeleton(&f.body, 1);
    }
}

struct Cx<'a> {
    locals: Vec<Type>,
    /// How many times each local is read in the function.
    reads: Vec<usize>,
    shapes: &'a [Shape],
    name: &'a str,
}

/// A loop that builds list `acc` from the items of local `src`, which is
/// set to `list`: `{ let src = list; var acc = []; for var in src { … } acc }`.
struct Producer<'b> {
    src: Local,
    list: &'b Expr,
    acc: Local,
    var: Local,
    index: Local,
    body: &'b Block,
}

impl Cx<'_> {
    fn block(&mut self, b: &mut Block) {
        // A chain written as one expression nests each stage's block
        // inside the next's first `let`. Hoisting the inner block into
        // its own statement turns the chain into the statement pairs
        // the pairwise fusion below matches.
        let mut i = 0;
        while i < b.stmts.len() {
            if let Some(hoisted) = self.hoist(&mut b.stmts[i]) {
                // The hoisted stages may hold more.
                b.stmts.splice(i..i, hoisted);
                continue;
            }
            i += 1;
        }
        for s in &mut b.stmts {
            match s {
                Stmt::While { body, .. }
                | Stmt::ForRange { body, .. }
                | Stmt::ForEach { body, .. } => self.block(body),
                // A loop written as an expression (a chain stage holding
                // a nested chain in its body).
                Stmt::Let(_, e) | Stmt::Expr(e) => {
                    if let ExprKind::Block(inner) = &mut e.kind {
                        self.block(inner);
                    }
                }
                _ => {}
            }
        }
        let mut i = 0;
        while i + 1 < b.stmts.len() {
            match self.fuse(&b.stmts[i], &b.stmts[i + 1]) {
                // The fused statement may fuse with the next one too.
                Some(fused) => {
                    b.stmts[i + 1] = fused;
                    b.stmts.remove(i);
                }
                None => i += 1,
            }
        }
    }

    /// `next` with `stmt`'s list fused into it, when `stmt` is
    /// `let a = <producer>` and `next` starts with a loop over `a`.
    fn fuse(&mut self, stmt: &Stmt, next: &Stmt) -> Option<Stmt> {
        let Stmt::Let(a, made) = stmt else {
            return None;
        };
        if self.reads[*a as usize] != 1 {
            return None;
        }
        let p = producer(match &made.kind {
            ExprKind::Fallback(fast, _) => fast,
            ExprKind::Block(b) => b,
            _ => return None,
        })?;
        let mut next = next.clone();
        let consumer = first_evaluated(&mut next)?;
        let fast = self.fused(&p, *a, consumer)?;
        let slow = Block {
            stmts: vec![stmt.clone()],
            value: Some(Box::new(consumer.clone())),
        };
        if std::env::var_os("KAVACH_FUSE_LOG").is_some() {
            eprintln!("fuse: {} line {}", self.name, consumer.line);
        }
        consumer.kind = ExprKind::Fallback(fast, slow);
        Some(next)
    }

    /// The fast block: `consumer`'s loop over `a` run inside `p`'s loop,
    /// for each item `p` would push. Its locals are new ones.
    fn fused(&mut self, p: &Producer, a: Local, consumer: &Expr) -> Option<Block> {
        let ExprKind::Block(c) = &consumer.kind else {
            return None;
        };
        let [
            Stmt::Let(src2, from),
            lets @ ..,
            Stmt::ForEach {
                var: var2,
                index: index2,
                list,
                body: body2,
            },
        ] = &c.stmts[..]
        else {
            return None;
        };
        let constant = |e: &Expr| match &e.kind {
            ExprKind::List(items) => items.is_empty(),
            kind => matches!(
                kind,
                ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::None
            ),
        };
        if !matches!(from.kind, ExprKind::Local(x) | ExprKind::Move(x) if x == a)
            || list != src2
            || !lets
                .iter()
                .all(|s| matches!(s, Stmt::Let(_, e) if constant(e)))
            || mentions(body2, &[*src2, *index2])
            || c.value
                .as_ref()
                .is_some_and(|v| mentions_expr(v, &[*src2, *index2, *var2]))
        {
            return None;
        }
        // Each push of an item becomes the consumer's loop body for it.
        let mut body = p.body.clone();
        exprs_mut(&mut body, &mut |e| {
            if let ExprKind::Assign(l, value) = &e.kind
                && let (true, ExprKind::Rt(RtOp::ListPush, args)) = (*l == p.acc, &value.kind)
            {
                let mut stmts = vec![Stmt::Let(*var2, args[1].clone())];
                stmts.extend(body2.stmts.iter().cloned());
                e.kind = ExprKind::Block(Block { stmts, value: None });
            }
        });
        let mut stmts = vec![Stmt::Let(p.src, p.list.clone())];
        stmts.extend(lets.iter().cloned());
        stmts.push(Stmt::ForEach {
            var: p.var,
            index: p.index,
            list: p.src,
            body,
        });
        let mut fast = Block {
            stmts,
            value: c.value.clone(),
        };
        if mentions(&fast, &[p.acc]) || body2.value.is_some() || !self.pure(&fast) {
            return None;
        }
        // Moves are worked out again later (`moves.rs`); the code as
        // written may still need what the fast block reads.
        exprs_mut(&mut fast, &mut |e| {
            if let ExprKind::Move(l) = e.kind {
                e.kind = ExprKind::Local(l);
            }
        });
        let mut own = Vec::new();
        defined(&fast, &mut own);
        let renamed: Vec<(Local, Local)> = own
            .iter()
            .map(|l| {
                self.locals.push(self.locals[*l as usize]);
                self.reads.push(0);
                (*l, (self.locals.len() - 1) as Local)
            })
            .collect();
        locals_mut(&mut fast, &mut |l| {
            if let Some((_, to)) = renamed.iter().find(|(from, _)| from == l) {
                *l = *to;
            }
        });
        Some(fast)
    }

    /// Whether `b` has no effects but setting its own locals and failing
    /// a check: no calls, printing, early exits or runtime calls that may
    /// fail inside the runtime.
    fn pure(&self, b: &Block) -> bool {
        let mut own = Vec::new();
        defined(b, &mut own);
        let shape = |t: Type| match t {
            Type::Ref(id) => self.shapes.get(id as usize),
            _ => None,
        };
        let is_list = |t| matches!(shape(t), Some(Shape::List(_)));
        let is_text = |t| matches!(shape(t), Some(Shape::Str));
        plain_stmts(b)
            && !any_in_block(b, &mut |e| match &e.kind {
                ExprKind::Int(_)
                | ExprKind::Float(_)
                | ExprKind::Bool(_)
                | ExprKind::Char(_)
                | ExprKind::Str(_)
                | ExprKind::Local(_)
                | ExprKind::Move(_)
                | ExprKind::Fast(..)
                | ExprKind::And(..)
                | ExprKind::Or(..)
                | ExprKind::Panic(_)
                | ExprKind::Format(_)
                | ExprKind::Tuple(_)
                | ExprKind::Record { .. }
                | ExprKind::GetField(..)
                | ExprKind::List(_)
                | ExprKind::None
                | ExprKind::Some(_)
                | ExprKind::IsSome(_) => false,
                ExprKind::If(_, then, otherwise) => !plain_stmts(then) || !plain_stmts(otherwise),
                ExprKind::Block(x) => !plain_stmts(x),
                ExprKind::Assign(l, _) => !own.contains(l),
                ExprKind::Unary(_, x) => x.ty.is_ref(),
                ExprKind::Binary(_, x, _) => x.ty.is_ref() && !is_text(x.ty),
                ExprKind::Convert(x, _) => !(e.ty.is_float() && x.ty.is_int()),
                ExprKind::Index(base, _) => !is_list(base.ty),
                ExprKind::Rt(op, _) => !matches!(
                    op,
                    RtOp::StrLen
                        | RtOp::StrIsEmpty
                        | RtOp::ListLen
                        | RtOp::ListIsEmpty
                        | RtOp::ListPush
                ),
                _ => true,
            })
    }
}

/// `b` as a producer (see [`Producer`]): its only use of `acc` is one
/// `acc = acc.push(item)`.
fn producer(b: &Block) -> Option<Producer<'_>> {
    let [
        Stmt::Let(src, list),
        Stmt::Let(acc, start),
        Stmt::ForEach {
            var,
            index,
            list: over,
            body,
        },
    ] = &b.stmts[..]
    else {
        return None;
    };
    let value = b.value.as_deref()?;
    let is =
        |e: &Expr, l: Local| matches!(e.kind, ExprKind::Local(x) | ExprKind::Move(x) if x == l);
    if !matches!(&start.kind, ExprKind::List(items) if items.is_empty())
        || !matches!(list.kind, ExprKind::Local(_) | ExprKind::Move(_))
        || over != src
        || !is(value, *acc)
        || mentions(body, &[*src])
    {
        return None;
    }
    let (mut pushes, mut uses) = (0, 0);
    any_in_block(body, &mut |e| {
        match &e.kind {
            ExprKind::Assign(l, v) if *l == *acc => {
                pushes += 1;
                if matches!(&v.kind, ExprKind::Rt(RtOp::ListPush, args) if is(&args[0], *acc)) {
                    uses += 1;
                }
            }
            ExprKind::Local(l) | ExprKind::Move(l) if *l == *acc => uses += 1,
            _ => {}
        }
        false
    });
    // One push, and `acc` read only as the list pushed onto.
    (pushes == 1 && uses == 2).then_some(Producer {
        src: *src,
        list,
        acc: *acc,
        var: *var,
        index: *index,
        body,
    })
}

/// The expression `s` evaluates first, when it is all of a `let`, an
/// expression statement, or the first value printed.
fn first_evaluated(s: &mut Stmt) -> Option<&mut Expr> {
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) => Some(e),
        Stmt::Print(parts) => parts.iter_mut().find_map(|p| match p {
            PrintPart::Value(e) => Some(e),
            PrintPart::Text(_) => None,
        }),
        _ => None,
    }
}

/// Whether `b`'s statements, at any depth, are only `let`, expressions
/// and loops (no printing, `return`, `break`, `continue` or checks).
fn plain_stmts(b: &Block) -> bool {
    b.stmts.iter().all(|s| match s {
        Stmt::Let(..) | Stmt::Expr(_) => true,
        Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
            plain_stmts(body)
        }
        _ => false,
    })
}

/// The locals `b` sets up: `let`s and loop variables, at any depth.
fn defined(b: &Block, out: &mut Vec<Local>) {
    for s in &b.stmts {
        match s {
            Stmt::Let(l, _) => out.push(*l),
            Stmt::LetResult { value, error, .. } => out.extend([*value, *error]),
            Stmt::ForRange { var, body, .. } => {
                out.push(*var);
                defined(body, out);
            }
            Stmt::ForEach {
                var,
                index,
                list,
                body,
            } => {
                out.extend([*var, *index, *list]);
                defined(body, out);
            }
            Stmt::While { body, .. } => defined(body, out),
            _ => {}
        }
    }
    any_in_block(b, &mut |e| {
        match &e.kind {
            ExprKind::If(_, x, y) | ExprKind::Fallback(x, y) => {
                defined(x, out);
                defined(y, out);
            }
            ExprKind::Block(x) => defined(x, out),
            _ => {}
        }
        false
    });
    out.sort_unstable();
    out.dedup();
}

/// Whether `b` reads, moves or assigns any of `ls`.
fn mentions(b: &Block, ls: &[Local]) -> bool {
    any_in_block(b, &mut |e| touches(e, ls))
}

fn mentions_expr(e: &Expr, ls: &[Local]) -> bool {
    any_expr(e, &mut |e| touches(e, ls))
}

fn touches(e: &Expr, ls: &[Local]) -> bool {
    matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) | ExprKind::Assign(l, _) if ls.contains(&l))
}

mod hoist;
#[cfg(test)]
mod tests;
