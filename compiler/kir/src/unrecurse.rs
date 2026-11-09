//! Self-recursion into a loop: `fn down(n) = if c { base } else { e +
//! down(step) }` sums `e` in an accumulator instead of a call per round,
//! the transformation C++ compilers do. The loop is the fast half of a
//! `Fallback`: its round counter is bounded by a check, so a recursion
//! that would have run out of stack falls back to the code as written
//! and panics there (spec 3.8), and a check that fails in the loop (an
//! overflow met in another order) reruns the code as written too.

use crate::ir::*;

mod counted;

fn kind_name(k: &ExprKind) -> &'static str {
    match k {
        ExprKind::Block(_) => "a block value",
        ExprKind::Fallback(..) => "a fallback value",
        ExprKind::Call(..) => "a call value",
        ExprKind::Binary(..) => "a binary value",
        ExprKind::If(..) => "an if value",
        ExprKind::Local(_) | ExprKind::Move(_) => "a local value",
        _ => "another value shape",
    }
}

/// Far past any recursion depth a stack survives, far below forever.
const MAX_ROUNDS: i128 = 50_000_000;

pub fn unrecurse(fid: FuncId, f: &mut Function) {
    let log = std::env::var_os("KAVACH_UNRECURSE_LOG").is_some();
    let bail = |why: &str| {
        if log {
            eprintln!("unrecurse: a function bails: {why}");
        }
    };
    if f.fails || f.external.is_some() || f.ret != Type::I64 || !f.body.stmts.is_empty() {
        bail("shape");
        return;
    }
    let Some(value) = &f.body.value else {
        bail("no value");
        return;
    };
    let line = value.line;
    let ExprKind::If(c, a, b) = &value.kind else {
        bail(kind_name(&value.kind));
        return;
    };
    if !pure(c) {
        bail("condition");
        return;
    }
    // One branch is the base case, the other adds to one self-call.
    let (base, rec, flipped) = match (recursive(a, fid), recursive(b, fid)) {
        (None, Some(r)) => (a, r, false),
        (Some(r), None) => (b, r, true),
        _ => {
            if log {
                let shape = |blk: &Block| {
                    format!(
                        "{} stmts, value {}",
                        blk.stmts.len(),
                        blk.value.as_deref().map_or("none", |v| kind_name(&v.kind))
                    )
                };
                eprintln!(
                    "unrecurse: {} bails: branches (then: {}; else: {})",
                    f.name,
                    shape(a),
                    shape(b)
                );
            }
            return;
        }
    };
    let (each, args) = rec;
    if !base.stmts.is_empty() || !base.value.as_deref().is_some_and(pure) {
        bail("base");
        return;
    }
    if !each.is_none_or(pure) || !args.iter().all(pure) || args.len() != f.params.len() {
        bail("each or args");
        return;
    }
    let base_value = base.value.as_deref().expect("a pure base");
    if let Some((fast, locals)) = counted::counted(f, (c, flipped), base_value, each, args) {
        if log {
            eprintln!("unrecurse: {} becomes a counted loop", f.name);
        }
        let slow = Block {
            stmts: Vec::new(),
            value: f.body.value.take(),
        };
        f.locals = locals;
        f.body = Block {
            stmts: Vec::new(),
            value: Some(Box::new(Expr {
                kind: ExprKind::Fallback(fast, slow),
                ty: Type::I64,
                line,
            })),
        };
        return;
    }
    if log {
        eprintln!("unrecurse: {} becomes a loop", f.name);
    }

    let int = |n: i128| Expr {
        kind: ExprKind::Int(n),
        ty: Type::I64,
        line,
    };
    let local = |l: Local, ty: Type| Expr {
        kind: ExprKind::Local(l),
        ty,
        line,
    };
    let assign = |l: Local, v: Expr| {
        Stmt::Expr(Expr {
            kind: ExprKind::Assign(l, Box::new(v)),
            ty: Type::Unit,
            line,
        })
    };
    let add = |x: Expr, y: Expr| Expr {
        kind: ExprKind::Binary(BinOp::Add, Box::new(x), Box::new(y)),
        ty: Type::I64,
        line,
    };
    let mut locals = f.locals.clone();
    let fresh = |ty: Type, locals: &mut Vec<Type>| {
        locals.push(ty);
        (locals.len() - 1) as Local
    };
    let acc = fresh(Type::I64, &mut locals);
    let rounds = fresh(Type::I64, &mut locals);
    // The loop counts on copies of the parameters: the code as written,
    // run when the loop gives up, starts again from the arguments.
    let params = f.params.clone();
    let copies: Vec<Local> = params
        .iter()
        .map(|p| fresh(locals[*p as usize], &mut locals))
        .collect();
    let on_copies = |e: &Expr| -> Expr {
        let mut b = Block {
            stmts: Vec::new(),
            value: Some(Box::new(e.clone())),
        };
        crate::walk::locals_mut(&mut b, &mut |l| {
            if let Some(i) = params.iter().position(|p| p == l) {
                *l = copies[i];
            }
        });
        *b.value.expect("the value put in")
    };
    let mut body = Vec::new();
    body.push(assign(rounds, add(local(rounds, Type::I64), int(1))));
    body.push(Stmt::Check {
        cond: Expr {
            kind: ExprKind::Binary(
                BinOp::Lt,
                Box::new(local(rounds, Type::I64)),
                Box::new(int(MAX_ROUNDS)),
            ),
            ty: Type::Bool,
            line,
        },
        message: "deep recursion".to_string(),
        line,
    });
    if let Some(e) = each {
        body.push(assign(acc, add(local(acc, Type::I64), on_copies(e))));
    }
    // The next round's arguments, all from this round's.
    let temps: Vec<Local> = args
        .iter()
        .map(|x| {
            let t = fresh(x.ty, &mut locals);
            body.push(Stmt::Let(t, on_copies(x)));
            t
        })
        .collect();
    for (p, t) in copies.iter().zip(&temps) {
        body.push(assign(*p, local(*t, locals[*t as usize])));
    }
    let cond = on_copies(c);
    let stop = Expr {
        kind: ExprKind::Unary(UnOp::Not, Box::new(cond)),
        ty: Type::Bool,
        line,
    };
    let stop = if flipped {
        // The base case was the `else` branch: loop while the condition
        // holds.
        *stop_inner(stop)
    } else {
        stop
    };
    let mut start: Vec<Stmt> = params
        .iter()
        .zip(&copies)
        .map(|(p, c)| Stmt::Let(*c, local(*p, locals[*p as usize])))
        .collect();
    start.extend([Stmt::Let(acc, int(0)), Stmt::Let(rounds, int(0))]);
    let fast = Block {
        stmts: start
            .into_iter()
            .chain([Stmt::While {
                cond: stop,
                body: Block {
                    stmts: body,
                    value: None,
                },
            }])
            .collect(),
        value: Some(Box::new(add(
            local(acc, Type::I64),
            on_copies(base.value.as_deref().expect("a pure base")),
        ))),
    };
    let slow = Block {
        stmts: Vec::new(),
        value: f.body.value.take(),
    };
    f.locals = locals;
    f.body = Block {
        stmts: Vec::new(),
        value: Some(Box::new(Expr {
            kind: ExprKind::Fallback(fast, slow),
            ty: Type::I64,
            line,
        })),
    };
}

/// `not (not c)` back to `c`, for a flipped branch pair.
fn stop_inner(stop: Expr) -> Box<Expr> {
    match stop.kind {
        ExprKind::Unary(UnOp::Not, inner) => inner,
        _ => Box::new(stop),
    }
}

/// `b` as `(each, args)` when its value is `each + self(args)`,
/// `self(args) + each` or a bare `self(args)`.
#[allow(clippy::type_complexity)]
fn recursive(b: &Block, fid: FuncId) -> Option<(Option<&Expr>, &[Expr])> {
    if !b.stmts.is_empty() {
        return None;
    }
    let mut value = b.value.as_deref()?;
    // Lowering sometimes wraps a branch in a bare block.
    while let ExprKind::Block(inner) = &value.kind {
        if !inner.stmts.is_empty() {
            return None;
        }
        value = inner.value.as_deref()?;
    }
    let call = |e: &Expr| matches!(&e.kind, ExprKind::Call(id, _) if *id == fid);
    match &value.kind {
        ExprKind::Call(id, args) if *id == fid => Some((None, args)),
        ExprKind::Binary(BinOp::Add, x, y) if call(y) && !call(x) => {
            let ExprKind::Call(_, args) = &y.kind else {
                return None;
            };
            Some((Some(x), args))
        }
        ExprKind::Binary(BinOp::Add, x, y) if call(x) && !call(y) => {
            let ExprKind::Call(_, args) = &x.kind else {
                return None;
            };
            Some((Some(y), args))
        }
        _ => None,
    }
}

/// Only arithmetic, comparisons and scalar reads: running it again, or
/// in another round, changes nothing.
fn pure(e: &Expr) -> bool {
    if e.ty.is_ref() {
        return false;
    }
    let kids = |xs: &[Expr]| xs.iter().all(pure);
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Char(_) => true,
        ExprKind::Local(l) | ExprKind::Move(l) => {
            let _ = l;
            true
        }
        ExprKind::Unary(_, x) | ExprKind::Convert(x, _) => pure(x),
        ExprKind::Binary(op, x, y) => {
            // Division and remainder can panic on zero; the fallback
            // would rerun everything, so leave them out.
            !matches!(op, BinOp::Div) && pure(x) && pure(y)
        }
        ExprKind::And(x, y) | ExprKind::Or(x, y) | ExprKind::InRange(x, y) => pure(x) && pure(y),
        ExprKind::Fast(_, x, y) => pure(x) && pure(y),
        ExprKind::Tuple(xs) => kids(xs),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(kind: ExprKind, ty: Type) -> Expr {
        Expr { kind, ty, line: 1 }
    }

    /// `fn down(n) = if n == 0 { 7 } else { n + down(n - 1) }`.
    fn down() -> Function {
        let local = |l: u32| e(ExprKind::Local(l), Type::I64);
        let int = |n: i128| e(ExprKind::Int(n), Type::I64);
        let bin = |op: BinOp, a: Expr, b: Expr, ty: Type| {
            e(ExprKind::Binary(op, Box::new(a), Box::new(b)), ty)
        };
        let cond = bin(BinOp::Eq, local(0), int(0), Type::Bool);
        let base = Block {
            stmts: vec![],
            value: Some(Box::new(int(7))),
        };
        let call = e(
            ExprKind::Call(0, vec![bin(BinOp::Sub, local(0), int(1), Type::I64)]),
            Type::I64,
        );
        let rec = Block {
            stmts: vec![],
            value: Some(Box::new(bin(BinOp::Add, local(0), call, Type::I64))),
        };
        Function {
            name: "down".into(),
            params: vec![0],
            ret: Type::I64,
            locals: vec![Type::I64],
            body: Block {
                stmts: vec![],
                value: Some(Box::new(e(
                    ExprKind::If(Box::new(cond), base, rec),
                    Type::I64,
                ))),
            },
            line: 1,
            fails: false,
            external: None,
        }
    }

    #[test]
    fn the_loop_leaves_the_arguments_for_the_code_as_written() {
        let mut f = down();
        unrecurse(0, &mut f);
        let Some(ExprKind::Fallback(fast, _)) = f.body.value.as_deref().map(|v| &v.kind) else {
            panic!("a fallback");
        };
        // The bug (2026-11-04): the loop counted `n` down in place, so
        // when it gave up, the code as written started from where it
        // stopped, and `depth(60_000_000)` gave 10,000,001.
        assert!(!crate::walk::sets_local(fast, 0), "the loop changes `n`");
    }

    /// The halves of `f` after `unrecurse`: the loop, and the code as
    /// written, which still calls itself.
    fn halves(mut f: Function) -> (Block, Block) {
        unrecurse(0, &mut f);
        let Some(ExprKind::Fallback(fast, slow)) = f.body.value.map(|v| v.kind) else {
            panic!("a fallback");
        };
        assert!(crate::walk::any_in_block(&fast, &mut |x| matches!(
            x.kind,
            ExprKind::Int(MAX_ROUNDS)
        )));
        assert!(!crate::walk::any_in_block(&fast, &mut |x| matches!(
            x.kind,
            ExprKind::Call(..)
        )));
        assert!(crate::walk::any_in_block(&slow, &mut |x| matches!(
            x.kind,
            ExprKind::Call(0, _)
        )));
        (fast, slow)
    }

    #[test]
    fn counting_down_by_one_to_zero_becomes_a_counted_loop() {
        let (fast, _) = halves(down());
        assert!(
            fast.stmts
                .iter()
                .any(|s| matches!(s, Stmt::ForRange { .. }))
        );
        let checks = fast
            .stmts
            .iter()
            .filter(|s| matches!(s, Stmt::Check { .. }));
        assert_eq!(checks.count(), 2, "0 <= n and n <= MAX_ROUNDS");
    }

    #[test]
    fn another_adding_recursion_becomes_a_bounded_loop() {
        // `down(n - 2)`: no counted loop, a `while` with a round bound.
        let mut f = down();
        crate::walk::exprs_mut(&mut f.body, &mut |e| {
            if let ExprKind::Binary(BinOp::Sub, _, k) = &mut e.kind {
                k.kind = ExprKind::Int(2);
            }
        });
        let (fast, _) = halves(f);
        assert!(fast.stmts.iter().any(|s| matches!(s, Stmt::While { .. })));
    }

    #[test]
    fn a_recursion_with_no_base_case_is_left_alone() {
        let mut f = down();
        // Make both branches recursive: no base case.
        let Some(v) = f.body.value.as_mut() else {
            panic!()
        };
        let ExprKind::If(_, a, _) = &mut v.kind else {
            panic!()
        };
        a.value = Some(Box::new(Expr {
            kind: ExprKind::Call(
                0,
                vec![Expr {
                    kind: ExprKind::Int(1),
                    ty: Type::I64,
                    line: 1,
                }],
            ),
            ty: Type::I64,
            line: 1,
        }));
        unrecurse(0, &mut f);
        assert!(!matches!(
            f.body.value.as_deref().map(|v| &v.kind),
            Some(ExprKind::Fallback(..))
        ));
    }
}
