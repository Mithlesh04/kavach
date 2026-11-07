use super::jam_loops;
use crate::ir::*;

fn e(kind: ExprKind, ty: Type) -> Expr {
    Expr { kind, ty, line: 1 }
}

fn local(l: Local, ty: Type) -> Expr {
    e(ExprKind::Local(l), ty)
}

/// Locals: 0 `j`, 1 `k`, 2 `cell`, 3 `total`, 4 `row` ([Float]),
/// 5 `b` ([[Float]]), 6 `n`.
/// `for j in 0..n { let cell = 0.0; for k in 0..n { <inner> }; total += cell }`
fn matrix(inner: Stmt) -> Function {
    let (list, rows) = (Type::Ref(1), Type::Ref(2));
    let n = || local(6, Type::I64);
    let total = e(
        ExprKind::Assign(
            3,
            Box::new(e(
                ExprKind::Binary(
                    BinOp::Add,
                    Box::new(local(3, Type::F64)),
                    Box::new(local(2, Type::F64)),
                ),
                Type::F64,
            )),
        ),
        Type::Unit,
    );
    let body = Block {
        stmts: vec![
            Stmt::Let(2, e(ExprKind::Float(0.0), Type::F64)),
            Stmt::ForRange {
                var: 1,
                start: e(ExprKind::Int(0), Type::I64),
                end: n(),
                inclusive: false,
                body: Block {
                    stmts: vec![inner],
                    value: None,
                },
            },
            Stmt::Expr(total),
        ],
        value: None,
    };
    Function {
        name: "main".into(),
        params: vec![],
        ret: Type::Unit,
        locals: vec![
            Type::I64,
            Type::I64,
            Type::F64,
            Type::F64,
            list,
            rows,
            Type::I64,
        ],
        body: Block {
            stmts: vec![Stmt::ForRange {
                var: 0,
                start: e(ExprKind::Int(0), Type::I64),
                end: n(),
                inclusive: false,
                body,
            }],
            value: None,
        },
        line: 1,
        fails: false,
        external: None,
    }
}

/// `cell = cell + row[k] * b[k][j]`
fn multiply_add() -> Stmt {
    let index = |base: Expr, i: Local, ty| {
        e(
            ExprKind::Index(Box::new(base), Box::new(local(i, Type::I64))),
            ty,
        )
    };
    let row_k = index(local(4, Type::Ref(1)), 1, Type::F64);
    let b_k = index(local(5, Type::Ref(2)), 1, Type::Ref(1));
    let b_kj = index(b_k, 0, Type::F64);
    let product = e(
        ExprKind::Binary(BinOp::Mul, Box::new(row_k), Box::new(b_kj)),
        Type::F64,
    );
    let sum = e(
        ExprKind::Binary(BinOp::Add, Box::new(local(2, Type::F64)), Box::new(product)),
        Type::F64,
    );
    Stmt::Expr(e(ExprKind::Assign(2, Box::new(sum)), Type::Unit))
}

fn has_while(f: &Function) -> bool {
    f.body.stmts.iter().any(|s| matches!(s, Stmt::While { .. }))
}

#[test]
fn a_matrix_product_is_jammed_by_eight() {
    let mut f = matrix(multiply_add());
    jam_loops(&mut f);
    assert!(has_while(&f), "a loop over groups of eight");
    // The original loop stays for the rest, after the groups.
    assert!(matches!(
        f.body.stmts.last(),
        Some(Stmt::ForRange { var: 0, .. })
    ));
    // Eight copies of `cell`, each with its own new local.
    assert!(f.locals.iter().filter(|t| **t == Type::F64).count() >= 2 + 8);
}

#[test]
fn an_inner_loop_that_prints_is_not_jammed() {
    let mut f = matrix(Stmt::Print(vec![PrintPart::Text("x".into())]));
    jam_loops(&mut f);
    assert!(!has_while(&f));
}

#[test]
fn only_the_jammed_inner_loop_skips_its_index_checks() {
    let mut f = matrix(multiply_add());
    jam_loops(&mut f);
    let kinds = |s: &Stmt, in_range: bool| {
        let b = Block {
            stmts: vec![s.clone()],
            value: None,
        };
        crate::walk::any_in_block(&b, &mut |e| match e.kind {
            ExprKind::InRange(..) => in_range,
            ExprKind::Index(..) => !in_range,
            _ => false,
        })
    };
    let groups = f
        .body
        .stmts
        .iter()
        .find(|s| matches!(s, Stmt::While { .. }));
    assert!(groups.is_some_and(|s| kinds(s, true)));
    let rest = f.body.stmts.last().expect("the rest");
    assert!(kinds(rest, false) && !kinds(rest, true));
}
