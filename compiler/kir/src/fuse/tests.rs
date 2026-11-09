use super::fuse_loops;
use crate::ir::*;

const LIST: Type = Type::Ref(0);

fn e(kind: ExprKind, ty: Type) -> Expr {
    Expr { kind, ty, line: 1 }
}

fn local(l: Local, ty: Type) -> Expr {
    e(ExprKind::Local(l), ty)
}

fn assign(l: Local, value: Expr) -> Stmt {
    Stmt::Expr(e(ExprKind::Assign(l, Box::new(value)), Type::Unit))
}

/// `{ const src = <from>; let acc = <start>; for let in src { <body> }; acc }`
fn list_loop(from: Local, src: Local, acc: Local, var: Local, start: Expr, body: Stmt) -> Expr {
    list_loop_over(local(from, LIST), src, acc, var, start, body)
}

fn list_loop_over(from: Expr, src: Local, acc: Local, var: Local, start: Expr, body: Stmt) -> Expr {
    let ty = start.ty;
    e(
        ExprKind::Block(Block {
            stmts: vec![
                Stmt::Let(src, from),
                Stmt::Let(acc, start),
                Stmt::ForEach {
                    var,
                    index: var + 1,
                    list: src,
                    body: Block {
                        stmts: vec![body],
                        value: None,
                    },
                },
            ],
            value: Some(Box::new(local(acc, ty))),
        }),
        ty,
    )
}

/// Locals: 0 `items`, 1 `doubled`, 2–5 the `map` loop, 6 `total`,
/// 7–10 the `sum` loop.
/// `const doubled = items.map(x => x * 2); const total = doubled.sum(y => y)`,
/// then `extra`.
fn pipeline(sum_body: Stmt, extra: Vec<Stmt>) -> Function {
    let twice = e(
        ExprKind::Binary(
            BinOp::Mul,
            Box::new(local(4, Type::I64)),
            Box::new(e(ExprKind::Int(2), Type::I64)),
        ),
        Type::I64,
    );
    let push = e(
        ExprKind::Rt(RtOp::ListPush, vec![local(3, LIST), twice]),
        LIST,
    );
    let map = list_loop(0, 2, 3, 4, e(ExprKind::List(vec![]), LIST), assign(3, push));
    let sum = list_loop(1, 7, 8, 9, e(ExprKind::Int(0), Type::I64), sum_body);
    let mut stmts = vec![Stmt::Let(1, map), Stmt::Let(6, sum)];
    stmts.extend(extra);
    let i = Type::I64;
    Function {
        name: "main".into(),
        params: vec![0],
        ret: Type::Unit,
        locals: vec![LIST, LIST, LIST, LIST, i, i, i, LIST, i, i, i],
        body: Block { stmts, value: None },
        line: 1,
        fails: false,
        external: None,
    }
}

/// `total = total + y`
fn add_up() -> Stmt {
    let sum = e(
        ExprKind::Binary(
            BinOp::Add,
            Box::new(local(8, Type::I64)),
            Box::new(local(9, Type::I64)),
        ),
        Type::I64,
    );
    assign(8, sum)
}

fn fuse(mut f: Function) -> Function {
    fuse_loops(&mut f, &[Shape::List(Type::I64)]);
    f
}

#[test]
fn map_then_sum_becomes_one_loop_with_the_code_as_written_behind_it() {
    let f = fuse(pipeline(add_up(), vec![]));
    let [Stmt::Let(6, value)] = &f.body.stmts[..] else {
        panic!("one statement");
    };
    let ExprKind::Fallback(fast, slow) = &value.kind else {
        panic!("a fallback");
    };
    // The fast block pushes nothing; the slow block is the code as written.
    let pushes = |b: &Block| {
        crate::walk::any_in_block(b, &mut |e| {
            matches!(e.kind, ExprKind::Rt(RtOp::ListPush, _))
        })
    };
    assert!(!pushes(fast));
    assert!(pushes(slow));
    assert!(matches!(slow.stmts[..], [Stmt::Let(1, _)]));
    // Its own locals are new ones.
    assert!(!crate::walk::sets_local(fast, 2) && !crate::walk::sets_local(fast, 8));
}

#[test]
fn a_chain_written_as_one_expression_still_fuses() {
    // `const total = items.map(x => x * 2).sum(y => y)`: the map's block
    // nested inside the sum's first `const`, not its own statement.
    let twice = e(
        ExprKind::Binary(
            BinOp::Mul,
            Box::new(local(4, Type::I64)),
            Box::new(e(ExprKind::Int(2), Type::I64)),
        ),
        Type::I64,
    );
    let push = e(
        ExprKind::Rt(RtOp::ListPush, vec![local(3, LIST), twice]),
        LIST,
    );
    let map = list_loop(0, 2, 3, 4, e(ExprKind::List(vec![]), LIST), assign(3, push));
    let sum = list_loop_over(map, 7, 8, 9, e(ExprKind::Int(0), Type::I64), add_up());
    let i = Type::I64;
    let f = fuse(Function {
        name: "main".into(),
        params: vec![0],
        ret: Type::Unit,
        locals: vec![LIST, LIST, LIST, LIST, i, i, i, LIST, i, i, i],
        body: Block {
            stmts: vec![Stmt::Let(6, sum)],
            value: None,
        },
        line: 1,
        fails: false,
        external: None,
    });
    let [Stmt::Let(6, value)] = &f.body.stmts[..] else {
        panic!("one statement");
    };
    let ExprKind::Fallback(fast, _) = &value.kind else {
        panic!("a fallback");
    };
    assert!(!crate::walk::any_in_block(fast, &mut |e| {
        matches!(e.kind, ExprKind::Rt(RtOp::ListPush, _))
    }));
}

#[test]
fn a_list_that_is_read_again_is_kept() {
    let again = Stmt::Expr(local(1, LIST));
    let f = fuse(pipeline(add_up(), vec![again]));
    assert_eq!(f.body.stmts.len(), 3);
}

#[test]
fn a_loop_that_prints_is_not_fused() {
    let print = Stmt::Print(vec![PrintPart::Value(local(9, Type::I64))]);
    let f = fuse(pipeline(print, vec![]));
    assert_eq!(f.body.stmts.len(), 2);
}

#[test]
fn a_loop_that_changes_an_outside_local_is_not_fused() {
    let f = fuse(pipeline(assign(6, local(9, Type::I64)), vec![]));
    assert_eq!(f.body.stmts.len(), 2);
}
