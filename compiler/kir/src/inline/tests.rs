use super::{inline_calls, lift_returns};
use crate::ir::*;

fn e(kind: ExprKind, ty: Type) -> Expr {
    Expr { kind, ty, line: 1 }
}

fn int(n: i128) -> Expr {
    e(ExprKind::Int(n), Type::I64)
}

fn local(l: Local) -> Expr {
    e(ExprKind::Local(l), Type::I64)
}

/// `fn fib(n) { if n < 2 { return n }; fib(n - 1) + fib(n - 2) }`
fn fib() -> Function {
    let call = |k| {
        let arg = e(
            ExprKind::Binary(BinOp::Sub, Box::new(local(0)), Box::new(int(k))),
            Type::I64,
        );
        e(ExprKind::Call(0, vec![arg]), Type::I64)
    };
    let early = e(
        ExprKind::If(
            Box::new(e(
                ExprKind::Binary(BinOp::Lt, Box::new(local(0)), Box::new(int(2))),
                Type::Bool,
            )),
            Block {
                stmts: vec![Stmt::Return(Some(local(0)))],
                value: None,
            },
            Block::default(),
        ),
        Type::Unit,
    );
    Function {
        name: "fib".into(),
        params: vec![0],
        ret: Type::I64,
        locals: vec![Type::I64],
        body: Block {
            stmts: vec![Stmt::Expr(early)],
            value: Some(Box::new(e(
                ExprKind::Binary(BinOp::Add, Box::new(call(1)), Box::new(call(2))),
                Type::I64,
            ))),
        },
        line: 1,
        fails: false,
        external: None,
    }
}

#[test]
fn an_early_return_becomes_if_else() {
    let mut f = fib();
    lift_returns(&mut f.body, f.ret);
    assert!(f.body.stmts.is_empty());
    let value = f.body.value.as_ref().expect("a value");
    let ExprKind::If(_, then, otherwise) = &value.kind else {
        panic!("an if");
    };
    assert!(matches!(
        then.value.as_deref().map(|v| &v.kind),
        Some(ExprKind::Local(0))
    ));
    assert!(matches!(
        otherwise.value.as_deref().map(|v| &v.kind),
        Some(ExprKind::Binary(BinOp::Add, ..))
    ));
}

#[test]
fn a_recursive_function_is_inlined_into_itself_two_levels() {
    let mut functions = vec![fib()];
    inline_calls(&mut functions, &[]);
    let f = &functions[0];
    // Each level replaces every call with a copy of the body as it is so
    // far: 2 calls become 4, then 16. Each call left is behind its own
    // copy of the quick exit.
    let (mut calls, mut guarded) = (0, 0);
    crate::walk::any_in_block(&f.body, &mut |e| {
        calls += matches!(e.kind, ExprKind::Call(0, _)) as usize;
        if let ExprKind::If(_, then, otherwise) = &e.kind {
            let early = matches!(
                then.value.as_deref().map(|v| &v.kind),
                Some(ExprKind::Local(_))
            );
            let call = matches!(
                otherwise.value.as_deref().map(|v| &v.kind),
                Some(ExprKind::Call(0, _))
            );
            guarded += (early && call) as usize;
        }
        false
    });
    assert_eq!((calls, guarded), (16, 16));
}

#[test]
fn a_quick_exit_that_could_panic_is_not_copied() {
    // `if n - 1 < 2 { return n }`: the subtraction can overflow, so the
    // test stays inside the function and no call gets a copy.
    let mut f = fib();
    let Stmt::Expr(early) = &mut f.body.stmts[0] else {
        panic!("an early return");
    };
    let ExprKind::If(test, ..) = &mut early.kind else {
        panic!("an if");
    };
    let minus = e(
        ExprKind::Binary(BinOp::Sub, Box::new(local(0)), Box::new(int(1))),
        Type::I64,
    );
    **test = e(
        ExprKind::Binary(BinOp::Lt, Box::new(minus), Box::new(int(2))),
        Type::Bool,
    );
    let mut functions = vec![f];
    inline_calls(&mut functions, &[]);
    assert_eq!(guarded_calls(&functions[0]), 0);
}

/// The calls of `fib` that stand behind a copy of its quick exit:
/// `if test { early } else { fib(…) }`.
fn guarded_calls(f: &Function) -> usize {
    let mut guarded = 0;
    crate::walk::any_in_block(&f.body, &mut |e| {
        if let ExprKind::If(_, _, otherwise) = &e.kind {
            let call = matches!(
                otherwise.value.as_deref().map(|v| &v.kind),
                Some(ExprKind::Call(0, _))
            );
            guarded += call as usize;
        }
        false
    });
    guarded
}

/// `fn twice(x) { x + x }` and `fn main() { twice(<arg>) }`.
fn twice_of(arg: Expr) -> Vec<Function> {
    let ty = arg.ty;
    let read = || e(ExprKind::Local(0), ty);
    let twice = Function {
        name: "twice".into(),
        params: vec![0],
        ret: ty,
        locals: vec![ty],
        body: Block {
            stmts: Vec::new(),
            value: Some(Box::new(e(
                ExprKind::Binary(BinOp::Add, Box::new(read()), Box::new(read())),
                ty,
            ))),
        },
        line: 1,
        fails: false,
        external: None,
    };
    let main = Function {
        name: "main".into(),
        params: Vec::new(),
        ret: ty,
        locals: Vec::new(),
        body: Block {
            stmts: Vec::new(),
            value: Some(Box::new(e(ExprKind::Call(0, vec![arg]), ty))),
        },
        line: 1,
        fails: false,
        external: None,
    };
    vec![twice, main]
}

/// The `const`s of the block `main`'s call became.
fn inlined_lets(functions: &[Function]) -> usize {
    match functions[1].body.value.as_deref().map(|v| &v.kind) {
        Some(ExprKind::Block(b)) => b.stmts.len(),
        _ => panic!("an inlined call"),
    }
}

#[test]
fn a_constant_argument_is_written_in_place_of_the_parameter() {
    let mut functions = twice_of(int(21));
    inline_calls(&mut functions, &[]);
    assert_eq!(inlined_lets(&functions), 0);
}

#[test]
fn text_read_twice_keeps_its_let() {
    // Each read of a text constant would make a new text object.
    let mut functions = twice_of(e(ExprKind::Str("ab".into()), Type::Ref(0)));
    inline_calls(&mut functions, &[]);
    assert_eq!(inlined_lets(&functions), 1);
}

/// `fn down(n) = if n == 0 { 7 } else { n + down(n - 1) }`, which
/// `unrecurse.rs` turns into a loop, and `fn main() = down(5)`.
fn down_and_main() -> Vec<Function> {
    let bin = |op: BinOp, a: Expr, b: Expr, ty: Type| {
        e(ExprKind::Binary(op, Box::new(a), Box::new(b)), ty)
    };
    let call = e(
        ExprKind::Call(0, vec![bin(BinOp::Sub, local(0), int(1), Type::I64)]),
        Type::I64,
    );
    let body = e(
        ExprKind::If(
            Box::new(bin(BinOp::Eq, local(0), int(0), Type::Bool)),
            Block {
                stmts: vec![],
                value: Some(Box::new(int(7))),
            },
            Block {
                stmts: vec![],
                value: Some(Box::new(bin(BinOp::Add, local(0), call, Type::I64))),
            },
        ),
        Type::I64,
    );
    let f = |name: &str, params: Vec<Local>, locals: Vec<Type>, value: Expr| Function {
        name: name.into(),
        params,
        ret: Type::I64,
        locals,
        body: Block {
            stmts: vec![],
            value: Some(Box::new(value)),
        },
        line: 1,
        fails: false,
        external: None,
    };
    let mut down = f("down", vec![0], vec![Type::I64], body);
    crate::unrecurse::unrecurse(0, &mut down);
    let main = f(
        "main",
        vec![],
        vec![],
        e(ExprKind::Call(0, vec![int(5)]), Type::I64),
    );
    vec![down, main]
}

/// A recursion turned loop is inlined as its loop, falling back to a call,
/// so the caller's arguments bound the loop; but never into its own code
/// as written, which would run the loop again at every level.
#[test]
fn a_recursion_turned_loop_is_inlined_as_its_loop() {
    let mut fs = down_and_main();
    inline_calls(&mut fs, &[]);
    let fallback = |f: &Function| match f.body.value.as_deref().map(|v| &v.kind) {
        Some(ExprKind::Block(b)) => match b.value.as_deref().map(|v| &v.kind) {
            Some(ExprKind::Fallback(fast, slow)) => Some((fast.clone(), slow.clone())),
            _ => None,
        },
        Some(ExprKind::Fallback(fast, slow)) => Some((fast.clone(), slow.clone())),
        _ => None,
    };
    let calls =
        |b: &Block| crate::walk::any_in_block(b, &mut |x| matches!(x.kind, ExprKind::Call(0, _)));
    let (fast, slow) = fallback(&fs[1]).expect("main holds the loop");
    assert!(!calls(&fast) && calls(&slow));
    let (_, own_slow) = fallback(&fs[0]).expect("down is still a loop");
    assert!(!crate::walk::any_in_block(&own_slow, &mut |x| matches!(
        x.kind,
        ExprKind::Fallback(..)
    )));
}
