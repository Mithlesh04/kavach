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

/// Whether lowering `src` spreads a loop over a grid's rows.
fn spreads_rows(src: &str) -> bool {
    let c = kavach_driver::compile(src, "t.kav", Default::default());
    let p = crate::lower(&c).expect("lowers");
    p.functions.iter().any(|f| {
        crate::walk::any_in_block(&f.body, &mut |e| {
            matches!(e.kind, ExprKind::Rt(crate::rt_op::RtOp::ParForRows, _))
        })
    })
}

/// `grid[row * w + col]` in `on cores for row` goes on all cores, the
/// rows counted by their items; a loop not written `on cores` doesn't.
#[test]
fn a_grids_rows_are_spread_when_written_on_cores() {
    let src = |place: &str, w: &str| {
        format!(
            "fn shade(w: Int, h: Int) -> [Int] {{
    let grid: [Int] = List.capacity(w * h)
    for _ in 0..w * h {{
        grid.push(0)
    }}
    {place}for row in 0..h {{
        for col in 0..{w} {{
            grid[row * {w} + col] = row + col
        }}
    }}
    return grid
}}

fn main() {{
    print(shade(300, 300).len())
}}
"
        )
    };
    assert!(spreads_rows(&src("on cores ", "w")));
    assert!(!spreads_rows(&src("", "w")));
}

/// `on cores for (i, x) in xs.enumerate()` writing `out[i]` goes on all
/// cores as the counted loop it is; a loop not written `on cores` doesn't.
#[test]
fn an_enumerate_loop_writing_its_slot_is_spread_when_written_on_cores() {
    let spreads = |place: &str| {
        let src = format!(
            "fn scale(xs: [Int]) -> [Int] {{\n    let out: [Int] = List.capacity(xs.len())\n    for _ in 0..xs.len() {{\n        out.push(0)\n    }}\n    {place}for (i, x) in xs.enumerate() {{\n        out[i] = x * 3 + i\n    }}\n    return out\n}}\n\nfn main() {{\n    print(scale([1, 2, 3]).len())\n}}\n"
        );
        let c = kavach_driver::compile(&src, "t.kav", Default::default());
        let p = crate::lower(&c).expect("lowers");
        p.functions.iter().any(|f| {
            crate::walk::any_in_block(&f.body, &mut |e| {
                matches!(e.kind, ExprKind::Rt(crate::rt_op::RtOp::ParFor, _))
            })
        })
    };
    assert!(spreads("on cores "));
    assert!(!spreads(""));
}
