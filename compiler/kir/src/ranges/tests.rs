use super::prove_ranges;
use crate::ir::*;

pub(super) fn e(kind: ExprKind) -> Expr {
    Expr {
        kind,
        ty: Type::I64,
        line: 1,
    }
}

pub(super) fn int(n: i128) -> Expr {
    e(ExprKind::Int(n))
}

pub(super) fn local(l: Local) -> Expr {
    e(ExprKind::Local(l))
}

pub(super) fn bin(op: BinOp, a: Expr, b: Expr) -> Expr {
    e(ExprKind::Binary(op, Box::new(a), Box::new(b)))
}

pub(super) fn assign(l: Local, value: Expr) -> Stmt {
    Stmt::Expr(Expr {
        kind: ExprKind::Assign(l, Box::new(value)),
        ty: Type::Unit,
        line: 1,
    })
}

pub(super) fn function(params: Vec<Local>, locals: usize, stmts: Vec<Stmt>) -> Function {
    Function {
        name: "f".into(),
        params,
        ret: Type::Unit,
        locals: vec![Type::I64; locals],
        body: Block { stmts, value: None },
        line: 1,
        fails: false,
        external: None,
    }
}

/// The operations left in an expression, outermost first: `+` for a
/// checked add, `fast+` for a proven one, and so on.
pub(super) fn ops(x: &Expr, out: &mut Vec<String>) {
    match &x.kind {
        ExprKind::Binary(op, a, b) => {
            out.push(op.symbol().to_string());
            ops(a, out);
            ops(b, out);
        }
        ExprKind::Fast(op, a, b) => {
            out.push(format!("fast {op:?}"));
            ops(a, out);
            ops(b, out);
        }
        ExprKind::Assign(_, v) => ops(v, out),
        _ => {}
    }
}

pub(super) fn ops_of(s: &Stmt) -> Vec<String> {
    let mut out = Vec::new();
    match s {
        Stmt::Expr(x) | Stmt::Let(_, x) => ops(x, &mut out),
        Stmt::ForRange { body, .. } | Stmt::While { body, .. } => {
            for s in &body.stmts {
                out.extend(ops_of(s));
            }
        }
        _ => {}
    }
    out
}

/// `i * 7 % 13` divides in 32 bits while `i * 7` stays below 2³², and in
/// 64 bits past it.
#[test]
fn a_modular_sum_needs_no_division_and_no_checks() {
    let ops_for = |rounds: i128| {
        // var total = 0
        // for i in 0..rounds { total = (total + i * 7 % 13) % 1_000_000_007 }
        let (total, i) = (0, 1);
        let step = bin(
            BinOp::Rem,
            bin(
                BinOp::Add,
                local(total),
                bin(BinOp::Rem, bin(BinOp::Mul, local(i), int(7)), int(13)),
            ),
            int(1_000_000_007),
        );
        let mut f = function(
            vec![],
            2,
            vec![
                Stmt::Let(total, int(0)),
                Stmt::ForRange {
                    var: i,
                    start: int(0),
                    end: int(rounds),
                    inclusive: false,
                    body: Block {
                        stmts: vec![assign(total, step)],
                        value: None,
                    },
                },
            ],
        );
        prove_ranges(&mut f);
        ops_of(&f.body.stmts[1])
    };
    assert_eq!(
        ops_for(500_000_000),
        ["fast RemOnce", "fast Add", "fast RemSmall", "fast Mul"]
    );
    assert_eq!(
        ops_for(1_000_000_000),
        ["fast RemOnce", "fast Add", "fast RemPositive", "fast Mul"]
    );
}

#[test]
fn checks_stay_where_they_can_fail() {
    // fn f(p) { let x = 0; while … { x = x + 1 }; const y = p * 2; const z = p % 10 }
    let (p, x, y, z) = (0, 1, 2, 3);
    let mut f = function(
        vec![p],
        4,
        vec![
            Stmt::Let(x, int(0)),
            Stmt::While {
                cond: Expr {
                    kind: ExprKind::Bool(true),
                    ty: Type::Bool,
                    line: 1,
                },
                body: Block {
                    stmts: vec![assign(x, bin(BinOp::Add, local(x), int(1)))],
                    value: None,
                },
            },
            Stmt::Let(y, bin(BinOp::Mul, local(p), int(2))),
            Stmt::Let(z, bin(BinOp::Rem, local(p), int(10))),
        ],
    );
    prove_ranges(&mut f);
    assert_eq!(ops_of(&f.body.stmts[1]), ["+"]);
    assert_eq!(ops_of(&f.body.stmts[2]), ["*"]);
    // `p` may be negative, so `p % 10` stays a real remainder.
    assert_eq!(ops_of(&f.body.stmts[3]), ["%"]);
}

#[test]
fn a_remainder_of_a_smaller_value_disappears() {
    // for i in 0..10 { let j = i % 16 }
    let (i, j) = (0, 1);
    let mut f = function(
        vec![],
        2,
        vec![Stmt::ForRange {
            var: i,
            start: int(0),
            end: int(10),
            inclusive: false,
            body: Block {
                stmts: vec![Stmt::Let(j, bin(BinOp::Rem, local(i), int(16)))],
                value: None,
            },
        }],
    );
    prove_ranges(&mut f);
    assert!(ops_of(&f.body.stmts[0]).is_empty());
}

#[test]
fn a_branch_condition_narrows_a_local() {
    // fn f(n) { let r = if n < 2 { n } else { n - 1 } }
    // In the `else` branch `n >= 2`, so `n - 1` can't overflow.
    let (n, r) = (0, 1);
    let cond = Expr {
        kind: ExprKind::Binary(BinOp::Lt, Box::new(local(n)), Box::new(int(2))),
        ty: Type::Bool,
        line: 1,
    };
    let then = Block {
        stmts: vec![],
        value: Some(Box::new(local(n))),
    };
    let otherwise = Block {
        stmts: vec![],
        value: Some(Box::new(bin(BinOp::Sub, local(n), int(1)))),
    };
    let choice = e(ExprKind::If(Box::new(cond), then, otherwise));
    let mut f = function(vec![n], 2, vec![Stmt::Let(r, choice)]);
    prove_ranges(&mut f);
    let Stmt::Let(
        _,
        Expr {
            kind: ExprKind::If(_, _, otherwise),
            ..
        },
    ) = &f.body.stmts[0]
    else {
        panic!("an if");
    };
    let value = otherwise.value.as_ref().expect("a value");
    assert!(matches!(value.kind, ExprKind::Fast(FastOp::Sub, ..)));
}

#[test]
fn a_branch_that_changes_the_local_learns_nothing() {
    // fn f(n) { if n < 2 { } else { n = n - 5; let y = n - 1 } }
    let (n, y) = (0, 1);
    let cond = Expr {
        kind: ExprKind::Binary(BinOp::Lt, Box::new(local(n)), Box::new(int(2))),
        ty: Type::Bool,
        line: 1,
    };
    let otherwise = Block {
        stmts: vec![
            assign(n, bin(BinOp::Sub, local(n), int(5))),
            Stmt::Let(y, bin(BinOp::Sub, local(n), int(1))),
        ],
        value: None,
    };
    let choice = Expr {
        kind: ExprKind::If(Box::new(cond), Block::default(), otherwise),
        ty: Type::Unit,
        line: 1,
    };
    let mut f = function(vec![n], 2, vec![Stmt::Expr(choice)]);
    prove_ranges(&mut f);
    let mut ops_left = Vec::new();
    crate::walk::any_in_block(&f.body, &mut |x| {
        if let ExprKind::Binary(op, ..) = x.kind {
            ops_left.push(op);
        }
        false
    });
    assert_eq!(ops_left.iter().filter(|o| **o == BinOp::Sub).count(), 2);
}

/// A branch's condition holds until the branch first sets its local: the
/// statements before that run first.
#[test]
fn a_branch_condition_holds_until_the_local_is_set() {
    // fn f(x, w) { if x <= 100 { let y = x + 1; x = w; let z = x + 1 } }
    let (x, w, y, z) = (0, 1, 2, 3);
    let cond = Expr {
        kind: ExprKind::Binary(BinOp::Le, Box::new(local(x)), Box::new(int(100))),
        ty: Type::Bool,
        line: 1,
    };
    let then = Block {
        stmts: vec![
            Stmt::Let(y, bin(BinOp::Add, local(x), int(1))),
            assign(x, local(w)),
            Stmt::Let(z, bin(BinOp::Add, local(x), int(1))),
        ],
        value: None,
    };
    let choice = Expr {
        kind: ExprKind::If(Box::new(cond), then, Block::default()),
        ty: Type::Unit,
        line: 1,
    };
    let mut f = function(vec![x, w], 4, vec![Stmt::Expr(choice)]);
    prove_ranges(&mut f);
    let Stmt::Expr(Expr {
        kind: ExprKind::If(_, then, _),
        ..
    }) = &f.body.stmts[0]
    else {
        unreachable!()
    };
    assert_eq!(ops_of(&then.stmts[0]), ["fast Add"], "x <= 100 still holds");
    assert_eq!(ops_of(&then.stmts[2]), ["+"], "x was set to anything");
}
