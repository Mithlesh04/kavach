//! A remainder by a constant in a counted loop: `i % 7` in `for i in
//! 0..n`. Each round `i` grows by one, so `i % 7` goes 0, 1, …, 6, 0, …:
//! a counter that wraps gives it with an add and a compare, where the
//! remainder takes a multiply and shifts (C++ compilers keep those).
//!
//! It runs after the loops that run on all cores are split off
//! (`par::outline`), so a counter carried from round to round never stops
//! a loop from being split; each chunk's loop gets its own. Only a
//! remainder the range pass proved can't be negative (`FastOp::Rem…`) is
//! counted, so the counter gives exactly the same values.
//!
//! A loop whose body runs straight through (no branch, call or inner
//! loop) is left alone: LLVM may vectorize it, computing many rounds'
//! remainders at once, which a counter carried from round to round would
//! stop (the `crc` benchmark's build loop).

use crate::ir::*;
use crate::walk;

pub fn count_remainders(f: &mut Function) {
    let mut locals = std::mem::take(&mut f.locals);
    walk::blocks_mut(&mut f.body, &mut |b| in_block(b, &mut locals));
    f.locals = locals;
}

fn in_block(b: &mut Block, locals: &mut Vec<Type>) {
    let mut i = 0;
    while i < b.stmts.len() {
        let lets = counters(&mut b.stmts[i], locals);
        let n = lets.len();
        b.stmts.splice(i..i, lets);
        i += n + 1;
    }
}

/// For a counted loop: its remainders become counters, and the counters'
/// first values, set just before the loop, are given back.
fn counters(s: &mut Stmt, locals: &mut Vec<Type>) -> Vec<Stmt> {
    let Stmt::ForRange {
        var, start, body, ..
    } = s
    else {
        return Vec::new();
    };
    if straight(body) {
        return Vec::new();
    }
    let (var, line) = (*var, start.line);
    // The start, read again before the loop: a constant or a local.
    let first = match &start.kind {
        ExprKind::Int(n) => Ok(*n),
        ExprKind::Local(l) | ExprKind::Move(l) => Err(*l),
        _ => return Vec::new(),
    };
    let mut made: Vec<(i128, Local, Type)> = Vec::new();
    walk::exprs_mut(body, &mut |e| {
        let ExprKind::Fast(FastOp::RemOnce | FastOp::RemPositive | FastOp::RemSmall, x, y) =
            &e.kind
        else {
            return;
        };
        let (ExprKind::Local(l), ExprKind::Int(c)) = (&x.kind, &y.kind) else {
            return;
        };
        if *l != var || *c <= 1 {
            return;
        }
        let c = *c;
        let m = match made.iter().find(|(d, ..)| *d == c) {
            Some((_, m, _)) => *m,
            None => {
                let m = locals.len() as Local;
                locals.push(e.ty);
                made.push((c, m, e.ty));
                m
            }
        };
        e.kind = ExprKind::Local(m);
    });
    let at = |kind: ExprKind, ty: Type| Expr { kind, ty, line };
    let mut lets = Vec::new();
    let mut steps = Vec::new();
    for (c, m, ty) in made {
        let int = |n: i128| at(ExprKind::Int(n), ty);
        // One below the start's remainder: each round counts first.
        let init = match first {
            Ok(n) => int(n % c - 1),
            Err(l) => {
                let rem = ExprKind::Binary(
                    BinOp::Rem,
                    Box::new(at(ExprKind::Local(l), ty)),
                    Box::new(int(c)),
                );
                at(
                    ExprKind::Binary(BinOp::Sub, Box::new(at(rem, ty)), Box::new(int(1))),
                    ty,
                )
            }
        };
        lets.push(Stmt::Let(m, init));
        let local = || Box::new(at(ExprKind::Local(m), ty));
        let next = at(ExprKind::Fast(FastOp::Add, local(), Box::new(int(1))), ty);
        steps.push(Stmt::Expr(at(
            ExprKind::Assign(m, Box::new(next)),
            Type::Unit,
        )));
        let wrapped = at(
            ExprKind::Binary(BinOp::Eq, local(), Box::new(int(c))),
            Type::Bool,
        );
        let zero = Block {
            stmts: vec![Stmt::Expr(at(
                ExprKind::Assign(m, Box::new(int(0))),
                Type::Unit,
            ))],
            value: None,
        };
        let wrap = ExprKind::If(Box::new(wrapped), zero, Block::default());
        steps.push(Stmt::Expr(at(wrap, Type::Unit)));
    }
    body.stmts.splice(0..0, steps);
    lets
}

/// Whether `body` runs straight through, as vectorized loops do.
fn straight(body: &Block) -> bool {
    let branches = walk::any_in_block(body, &mut |e| {
        matches!(
            e.kind,
            ExprKind::If(..)
                | ExprKind::And(..)
                | ExprKind::Or(..)
                | ExprKind::Fallback(..)
                | ExprKind::Call(..)
                | ExprKind::CallValue(..)
        )
    });
    let loops = body.stmts.iter().any(|s| {
        matches!(
            s,
            Stmt::While { .. } | Stmt::ForRange { .. } | Stmt::ForEach { .. }
        )
    });
    !branches && !loops
}

#[cfg(test)]
mod tests {
    use super::count_remainders;
    use crate::ir::*;

    fn e(kind: ExprKind) -> Expr {
        Expr {
            kind,
            ty: Type::I64,
            line: 1,
        }
    }

    /// `for i in 3..100 { total = total + i % 7; if … {} }`, with `branch`.
    fn looped(branch: bool) -> Function {
        let rem = e(ExprKind::Binary(
            BinOp::Rem,
            Box::new(e(ExprKind::Local(1))),
            Box::new(e(ExprKind::Int(7))),
        ));
        let sum = e(ExprKind::Binary(
            BinOp::Add,
            Box::new(e(ExprKind::Local(0))),
            Box::new(rem),
        ));
        let mut stmts = vec![Stmt::Expr(Expr {
            kind: ExprKind::Assign(0, Box::new(sum)),
            ty: Type::Unit,
            line: 1,
        })];
        if branch {
            let cond = Expr {
                kind: ExprKind::Bool(true),
                ty: Type::Bool,
                line: 1,
            };
            let choice = ExprKind::If(Box::new(cond), Block::default(), Block::default());
            stmts.push(Stmt::Expr(Expr {
                kind: choice,
                ty: Type::Unit,
                line: 1,
            }));
        }
        let body = Block { stmts, value: None };
        let mut f = Function {
            name: "f".into(),
            params: Vec::new(),
            ret: Type::Unit,
            locals: vec![Type::I64; 2],
            body: Block {
                stmts: vec![
                    Stmt::Let(0, e(ExprKind::Int(0))),
                    Stmt::ForRange {
                        var: 1,
                        start: e(ExprKind::Int(3)),
                        end: e(ExprKind::Int(100)),
                        inclusive: false,
                        body,
                    },
                ],
                value: None,
            },
            line: 1,
            fails: false,
            external: None,
        };
        crate::ranges::prove_ranges(&mut f);
        count_remainders(&mut f);
        f
    }

    /// After the range pass proves `i % 7` never negative, it becomes a
    /// counter set to `2` before the loop and counted at each round's top.
    #[test]
    fn a_loop_counters_remainder_becomes_a_counter() {
        let f = looped(true);
        assert_eq!(f.locals.len(), 3, "one counter");
        assert!(
            matches!(&f.body.stmts[1], Stmt::Let(2, x) if matches!(x.kind, ExprKind::Int(2))),
            "{:?}",
            f.body.stmts[1]
        );
        let Stmt::ForRange { body, .. } = &f.body.stmts[2] else {
            panic!("the loop moved: {:?}", f.body.stmts);
        };
        let rems = crate::walk::any_in_block(body, &mut |x| {
            matches!(
                x.kind,
                ExprKind::Fast(FastOp::RemSmall | FastOp::RemPositive | FastOp::RemOnce, ..)
                    | ExprKind::Binary(BinOp::Rem, ..)
            )
        });
        assert!(!rems, "a remainder is left: {body:?}");
    }

    /// A loop that runs straight through is left to LLVM's vectorizer.
    #[test]
    fn a_straight_loop_keeps_its_remainder() {
        let f = looped(false);
        assert_eq!(f.locals.len(), 2, "no counter");
    }
}
