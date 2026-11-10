//! Tests of accumulators in counted loops (`accumulate.rs`).

use crate::ir::*;
use crate::ranges::prove_ranges;
use crate::ranges::tests::{assign, bin, function, int, local, ops, ops_of};

/// A counted loop's accumulator stays within its start plus the rounds
/// times what a round adds (`accumulate.rs`), so its add needs no check:
/// `total` gains at most 6 a round for 200 million rounds. A loop with
/// too many rounds for the type keeps the check.
#[test]
fn a_counted_loops_accumulator_needs_no_check() {
    let counted = |rounds: i128| {
        // var total = 0
        // for i in 0..rounds { const x = i % 7; total = total + x }
        let (total, i, x) = (0, 1, 2);
        let mut f = function(
            vec![],
            3,
            vec![
                Stmt::Let(total, int(0)),
                Stmt::ForRange {
                    var: i,
                    start: int(0),
                    end: int(rounds),
                    inclusive: false,
                    body: Block {
                        stmts: vec![
                            Stmt::Let(x, bin(BinOp::Rem, local(i), int(7))),
                            assign(total, bin(BinOp::Add, local(total), local(x))),
                        ],
                        value: None,
                    },
                },
            ],
        );
        prove_ranges(&mut f);
        let Stmt::ForRange { body, .. } = &f.body.stmts[1] else {
            unreachable!()
        };
        ops_of(&body.stmts[1])
    };
    assert_eq!(counted(200_000_000), ["fast Add"]);
    assert_eq!(counted(i64::MAX as i128), ["+"]);
}

/// What a round adds may be a sum of bounded parts, one of them a mask of
/// the accumulator itself: `t & 1` is 0 or 1 whatever `t` is.
#[test]
fn an_accumulators_step_may_be_a_bounded_sum() {
    let counted = |rounds: i128| {
        // var total = 0
        // for i in 0..rounds { const x = i + (total & 1); total = total + (x + 3) }
        let (total, i, x) = (0, 1, 2);
        let mut f = function(
            vec![],
            3,
            vec![
                Stmt::Let(total, int(0)),
                Stmt::ForRange {
                    var: i,
                    start: int(0),
                    end: int(rounds),
                    inclusive: false,
                    body: Block {
                        stmts: vec![
                            Stmt::Let(
                                x,
                                bin(
                                    BinOp::Add,
                                    local(i),
                                    bin(BinOp::BitAnd, local(total), int(1)),
                                ),
                            ),
                            assign(
                                total,
                                bin(BinOp::Add, local(total), bin(BinOp::Add, local(x), int(3))),
                            ),
                        ],
                        value: None,
                    },
                },
            ],
        );
        prove_ranges(&mut f);
        let Stmt::ForRange { body, .. } = &f.body.stmts[1] else {
            unreachable!()
        };
        ops_of(&body.stmts[1])
    };
    assert_eq!(counted(100_000_000), ["fast Add", "fast Add"]);
    // 4 billion rounds of up to 4 billion: past `Int`'s limit.
    assert_eq!(counted(4_000_000_000)[0], "+");
}

/// A step that is a remainder by a constant is within it of 0, whatever
/// it divides: `total += i * k % 1000` with `k` a parameter. Also when
/// `version.rs` made two copies of the loop, one of which runs.
#[test]
fn a_remainder_bounds_a_step_in_either_copy_of_a_loop() {
    // fn f(k) { var total = 0; for i in 0..rounds { total = total + i * k % 1000 } }
    let (k, total, i) = (0, 1, 2);
    let lp = |rounds: i128| Stmt::ForRange {
        var: i,
        start: int(0),
        end: int(rounds),
        inclusive: false,
        body: Block {
            stmts: vec![assign(
                total,
                bin(
                    BinOp::Add,
                    local(total),
                    bin(BinOp::Rem, bin(BinOp::Mul, local(i), local(k)), int(1000)),
                ),
            )],
            value: None,
        },
    };
    let adds = |stmts: Vec<Stmt>| {
        let mut f = function(vec![k], 3, stmts);
        prove_ranges(&mut f);
        let mut out = Vec::new();
        crate::walk::any_in_block(&f.body, &mut |e| {
            if let ExprKind::Assign(_, v) = &e.kind {
                let mut seen = Vec::new();
                ops(v, &mut seen);
                out.push(seen.remove(0));
            }
            false
        });
        out
    };
    let one = |rounds| vec![Stmt::Let(total, int(0)), lp(rounds)];
    assert_eq!(adds(one(400_000_000)), ["fast Add"]);
    assert_eq!(adds(one(i64::MAX as i128)), ["+"]);
    let block = |s| Block {
        stmts: vec![s],
        value: None,
    };
    let pair = vec![
        Stmt::Let(total, int(0)),
        Stmt::Expr(Expr {
            kind: ExprKind::If(
                Box::new(bin(BinOp::Ge, local(k), int(0))),
                block(lp(400_000_000)),
                block(lp(400_000_000)),
            ),
            ty: Type::Unit,
            line: 1,
        }),
    ];
    assert_eq!(adds(pair), ["fast Add", "fast Add"]);
}

/// Past a `Check`, its condition holds for the rest of the block, and a
/// loop's rounds come from its bounds as the loop sees them: so a loop
/// to a checked parameter, inside another loop, still bounds its
/// accumulator. Without the checks, both adds keep theirs.
#[test]
fn checks_bound_what_follows_them() {
    let ops_with = |checked: bool| {
        // fn f(n) { for j in 0..10 {
        //     check n >= 0; check n <= 1000; const m = n + 1
        //     var total = 0
        //     for i in 1..=n { const x = i % 7; total = total + x }
        // } }
        let (n, total, i, x, j, m) = (0, 1, 2, 3, 4, 5);
        let check = |op, k| Stmt::Check {
            cond: Expr {
                kind: ExprKind::Binary(op, Box::new(local(n)), Box::new(int(k))),
                ty: Type::Bool,
                line: 1,
            },
            message: "deep recursion".into(),
            line: 1,
        };
        let mut stmts = Vec::new();
        if checked {
            stmts.extend([check(BinOp::Ge, 0), check(BinOp::Le, 1000)]);
        }
        stmts.extend([
            Stmt::Let(m, bin(BinOp::Add, local(n), int(1))),
            Stmt::Let(total, int(0)),
            Stmt::ForRange {
                var: i,
                start: int(1),
                end: local(n),
                inclusive: true,
                body: Block {
                    stmts: vec![
                        Stmt::Let(x, bin(BinOp::Rem, local(i), int(7))),
                        assign(total, bin(BinOp::Add, local(total), local(x))),
                    ],
                    value: None,
                },
            },
        ]);
        let outer = Stmt::ForRange {
            var: j,
            start: int(0),
            end: int(10),
            inclusive: false,
            body: Block { stmts, value: None },
        };
        let mut f = function(vec![n], 6, vec![outer]);
        prove_ranges(&mut f);
        let Stmt::ForRange { body, .. } = &f.body.stmts[0] else {
            unreachable!()
        };
        let at = body.stmts.len() - 3;
        let Stmt::ForRange { body: inner, .. } = &body.stmts[at + 2] else {
            unreachable!()
        };
        [ops_of(&body.stmts[at]), ops_of(&inner.stmts[1])]
    };
    assert_eq!(ops_with(true), [["fast Add"], ["fast Add"]]);
    assert_eq!(ops_with(false), [["+"], ["+"]]);
}
