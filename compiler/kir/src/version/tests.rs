use crate::ir::*;

/// For the loop versioned in program `src` (all passes run; `work` may be
/// inlined): the checks in its first copy, the signed `%`s left there,
/// and the checks in the loop as written, the last `else`.
fn versioned(src: &str) -> Option<(usize, usize, usize)> {
    let c = kavach_driver::compile(src, "t.kav", Default::default());
    let program = crate::lower(&c).expect("lowers");
    let all = Block {
        stmts: (program.functions.iter())
            .map(|f| {
                Stmt::Expr(Expr {
                    kind: ExprKind::Block(f.body.clone()),
                    ty: Type::Unit,
                    line: 0,
                })
            })
            .collect(),
        value: None,
    };
    let (mut first, mut last) = (None, None);
    crate::walk::any_in_block(&all, &mut |e| {
        if let ExprKind::If(cond, fast, written) = &e.kind
            && is_version_guard(cond)
        {
            first.get_or_insert_with(|| fast.clone());
            last = Some(written.clone());
        }
        false
    });
    let (fast, written) = (first?, last?);
    let mut signed = 0;
    crate::walk::any_in_block(&fast, &mut |e| {
        signed += usize::from(matches!(e.kind, ExprKind::Binary(BinOp::Rem, ..)));
        false
    });
    Some((super::checks(&fast), signed, super::checks(&written)))
}

/// Whether `cond` is the test `version.rs` puts before a copy:
/// `x >= 0 && x <= 2^31 - 1`, then perhaps `&& a >= 0` for accumulators.
fn is_version_guard(cond: &Expr) -> bool {
    match &cond.kind {
        ExprKind::And(a, b) => {
            let bound = |e: &Expr, op: BinOp, c: i128| {
                matches!(&e.kind, ExprKind::Binary(o, x, n)
                    if *o == op
                        && matches!(x.kind, ExprKind::Local(_))
                        && matches!(n.kind, ExprKind::Int(v) if v == c))
            };
            (bound(a, BinOp::Ge, 0) && bound(b, BinOp::Le, super::SMALL)) || is_version_guard(a)
        }
        _ => false,
    }
}

/// The loop of the `tasks` benchmark, with the rounds known or not.
fn work(rounds: &str, step: &str) -> String {
    format!(
        "fn work(seed: Int, n: Int) -> Int {{\n    let total = 0\n    for i in 0..{rounds} {{\n        total = {step}\n    }}\n    total\n}}\n\nfn main() {{\n    for s in [3, -3] {{\n        print(work(s, s + 7))\n    }}\n}}\n"
    )
}

#[test]
fn a_loop_multiplied_by_a_parameter_gets_a_copy_without_checks() {
    // Fewer rounds than `par.rs` splits off, so the copy stays one loop.
    let src = work("50_000", "(total + i * seed) % 1_000_000_007");
    let (fast, signed, written) = versioned(&src).expect("the loop is versioned");
    assert_eq!(fast, 0, "no check is left where 0 <= seed < 2^31");
    assert_eq!(
        signed, 0,
        "with locals of its own, `total` is never negative"
    );
    assert_eq!(written, 2, "the loop as written checks `*` and `+`");
}

#[test]
fn a_copy_with_its_own_locals_divides_without_a_sign() {
    // The rounds aren't known, so `i * seed` may overflow for any seed;
    // but where `seed` and `total` aren't negative, neither is the sum.
    let src = work("n", "(total + i * seed) % 1_000_000_007");
    assert_eq!(versioned(&src), Some((2, 0, 2)));
}

#[test]
fn a_loop_no_copy_helps_is_not_copied() {
    let src = work("n", "total + i * seed");
    assert_eq!(versioned(&src), None);
}
