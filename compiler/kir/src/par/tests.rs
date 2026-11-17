use super::outline;
use crate::ir::*;
use std::collections::HashSet;

fn e(kind: ExprKind, ty: Type, line: u32) -> Expr {
    Expr { kind, ty, line }
}

/// `fn sum(xs: [Int]) -> Int { var total = 0; for x in xs { total += x }; total }`,
/// with the `for` on line 3.
fn sum() -> (Vec<Function>, Vec<Shape>) {
    let list = Type::Ref(0);
    let add = e(
        ExprKind::Binary(
            BinOp::Add,
            Box::new(e(ExprKind::Local(1), Type::I64, 3)),
            Box::new(e(ExprKind::Local(3), Type::I64, 3)),
        ),
        Type::I64,
        3,
    );
    let body = Block {
        stmts: vec![
            Stmt::Let(1, e(ExprKind::Int(0), Type::I64, 2)),
            Stmt::Let(2, e(ExprKind::Local(0), list, 3)),
            Stmt::ForEach {
                var: 3,
                index: 4,
                list: 2,
                body: Block {
                    stmts: vec![Stmt::Expr(e(
                        ExprKind::Assign(1, Box::new(add)),
                        Type::Unit,
                        3,
                    ))],
                    value: None,
                },
            },
        ],
        value: Some(Box::new(e(ExprKind::Local(1), Type::I64, 4))),
    };
    let f = Function {
        name: "sum".into(),
        params: vec![0],
        ret: Type::I64,
        locals: vec![list, Type::I64, list, Type::I64, Type::I64],
        body,
        line: 1,
        fails: false,
        external: None,
        variadic: false,
    };
    (vec![f], vec![Shape::List(Type::I64)])
}

#[test]
fn a_walk_over_a_list_is_spread_only_when_written_on_cores() {
    // Another loop is `on cores`; this one, unmarked, stays on one core.
    for (lines, chunks) in [(vec![9], 0), (vec![3], 1)] {
        let (mut functions, mut shapes) = sum();
        outline(&mut functions, &mut shapes, &HashSet::from_iter(lines));
        assert_eq!(functions.len(), 1 + chunks);
    }
}
