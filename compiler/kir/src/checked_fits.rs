//! `x.checked_mul(y) ?? d`, `if let v = x.checked_add(y)` and the like on
//! `Int` (spec 5.6), without making the optional. An `Int?` is an object
//! on the heap, so a `checked_*` call cost an allocation and three runtime
//! calls (Sakshi's S14: 30 ns against 0.3 ns for `x * y`). When a `Let`
//! keeps such a result and the rest of its block only asks whether it is
//! there and unwraps it, the `Let` becomes whether the result fits
//! (`RtOp::CheckedFits`, one instruction in LLVM), and each unwrap the
//! plain operation where it fits. Where it doesn't, the unwrap is left as
//! it was, so it panics as before.

use crate::ir::{Block, Expr, ExprKind as K, Function, Local, Shape, Stmt, Type};
use crate::rt_op::RtOp;

/// Rewrites every such `Let` in `f`.
pub(crate) fn checked_fits(f: &mut Function, shapes: &[Shape]) {
    let mut locals = std::mem::take(&mut f.locals);
    crate::walk::blocks_mut(&mut f.body, &mut |b| {
        let mut i = 0;
        while i < b.stmts.len() {
            i += rewrite_at(b, i, &mut locals, shapes);
        }
    });
    f.locals = locals;
}

/// The operation's number for `CheckedFits`, and the one that wraps.
fn wrapping(op: RtOp) -> Option<(i128, RtOp)> {
    match op {
        RtOp::CheckedAdd => Some((0, RtOp::WrappingAdd)),
        RtOp::CheckedSub => Some((1, RtOp::WrappingSub)),
        RtOp::CheckedMul => Some((2, RtOp::WrappingMul)),
        _ => None,
    }
}

/// Rewrites statement `i` of `b` if it is such a `Let`; how many
/// statements to go on past.
fn rewrite_at(b: &mut Block, i: usize, locals: &mut Vec<Type>, shapes: &[Shape]) -> usize {
    let Stmt::Let(t, e) = &b.stmts[i] else {
        return 1;
    };
    let (t, opt) = (*t, e.ty);
    let K::Rt(op, args) = &e.kind else {
        return 1;
    };
    let Some((kind, wraps)) = wrapping(*op) else {
        return 1;
    };
    let ints = args.len() == 2 && args.iter().all(|a| a.ty == Type::I64);
    let optional_int = matches!(opt, Type::Ref(id)
        if matches!(shapes.get(id as usize), Some(Shape::Optional(Type::I64))));
    if !ints || !optional_int {
        return 1;
    }
    let mut rest = Block {
        stmts: b.stmts.split_off(i + 1),
        value: b.value.take(),
    };
    if !only_asked(&rest, t) {
        b.stmts.append(&mut rest.stmts);
        b.value = rest.value;
        return 1;
    }
    let Stmt::Let(_, e) = b.stmts.pop().expect("the `Let`") else {
        unreachable!("checked above")
    };
    let K::Rt(op, args) = e.kind else {
        unreachable!("checked above")
    };
    let line = e.line;
    let mut new_local = |ty: Type| {
        locals.push(ty);
        (locals.len() - 1) as Local
    };
    let (a, c, ok) = (
        new_local(Type::I64),
        new_local(Type::I64),
        new_local(Type::Bool),
    );
    let [x, y]: [Expr; 2] = args.try_into().expect("two operands");
    let local = |l: Local, ty: Type| Expr {
        kind: K::Local(l),
        ty,
        line,
    };
    b.stmts.push(Stmt::Let(a, x));
    b.stmts.push(Stmt::Let(c, y));
    let fits = K::Rt(
        RtOp::CheckedFits,
        vec![
            local(a, Type::I64),
            local(c, Type::I64),
            Expr {
                kind: K::Int(kind),
                ty: Type::I64,
                line,
            },
        ],
    );
    b.stmts.push(Stmt::Let(
        ok,
        Expr {
            kind: fits,
            ty: Type::Bool,
            line,
        },
    ));
    let operands = || vec![local(a, Type::I64), local(c, Type::I64)];
    crate::walk::exprs_mut(&mut rest, &mut |e| match &e.kind {
        K::IsSome(x) if is(x, t) => e.kind = K::Local(ok),
        K::Unwrap(x) if is(x, t) => {
            let done = Expr {
                kind: K::Rt(wraps, operands()),
                ty: Type::I64,
                line,
            };
            // Not there: the call as it was, which panics as it did.
            let checked = Expr {
                kind: K::Rt(op, operands()),
                ty: opt,
                line,
            };
            let panics = Expr {
                kind: K::Unwrap(Box::new(checked)),
                ty: Type::I64,
                line,
            };
            e.kind = K::If(
                Box::new(local(ok, Type::Bool)),
                Block {
                    stmts: Vec::new(),
                    value: Some(Box::new(done)),
                },
                Block {
                    stmts: Vec::new(),
                    value: Some(Box::new(panics)),
                },
            );
        }
        _ => {}
    });
    b.stmts.append(&mut rest.stmts);
    b.value = rest.value;
    3
}

/// Whether `x` reads local `t`.
fn is(x: &Expr, t: Local) -> bool {
    matches!(x.kind, K::Local(l) | K::Move(l) if l == t)
}

/// Whether `rest` reads `t` at least once, only to ask whether it is
/// there or to unwrap it, and never sets it.
fn only_asked(rest: &Block, t: Local) -> bool {
    if crate::walk::sets_local(rest, t) {
        return false;
    }
    let (mut all, mut asked) = (0, 0);
    crate::walk::any_in_block(rest, &mut |e| {
        match &e.kind {
            K::IsSome(x) | K::Unwrap(x) if is(x, t) => asked += 1,
            _ if is(e, t) => all += 1,
            _ => {}
        }
        false
    });
    asked > 0 && all == asked
}

#[cfg(test)]
mod tests;
