//! `a ?? b` through branches. After inlining, `a` is often a block or an
//! `if` whose branches end in `Some(x)` or `none`, as in
//! `j.int() ?? 0`, where `int()` is `match self { Integer(v) => v, _ =>
//! none }`. Then `??` moves into each branch: `Some(x)` becomes `x` and
//! `none` becomes `b`, so the optional is never built (a scalar inside
//! one would need a heap box).
//!
//! `??` is lowered as `let t = a; if t.is_some() { t.unwrap() } else { b }`
//! (`lower_expr.rs`). Only a fallback `b` that is a constant or a local is
//! moved, since it is copied into every branch. It is still read only on
//! the paths that end in `none`, and at the end of them, so what the
//! program sees doesn't change. A branch that ends in anything else keeps
//! the `let t = …; if …` form.

use crate::ir::*;
use crate::walk::exprs_mut;

pub fn coalesce(f: &mut Function) {
    let locals = &mut f.locals;
    exprs_mut(&mut f.body, &mut |e| {
        // When no branch ends in `Some(x)` or `none`, the `??` stays as
        // it is: moving it would only copy the fallback around.
        if let Some((mut value, fallback)) = split(e)
            && push(&mut value, &fallback, e.ty, locals)
        {
            *e = value;
        }
    });
}

/// For `let t = a; if t.is_some() { t.unwrap() } else { b }` with a
/// simple `b` of the same type as `a`'s inside: copies of `a` and `b`.
fn split(e: &Expr) -> Option<(Expr, Expr)> {
    let ExprKind::Block(block) = &e.kind else {
        return None;
    };
    let [Stmt::Let(tmp, value)] = block.stmts.as_slice() else {
        return None;
    };
    let tmp = *tmp;
    let Some(ExprKind::If(cond, then, other)) = block.value.as_deref().map(|v| &v.kind) else {
        return None;
    };
    let is_tmp = |x: &Expr| matches!(x.kind, ExprKind::Local(l) if l == tmp);
    let present = matches!(&cond.kind, ExprKind::IsSome(x) if is_tmp(x));
    let inside = then.stmts.is_empty()
        && matches!(then.value.as_deref(), Some(Expr { kind: ExprKind::Unwrap(x), ty, .. })
            if is_tmp(x) && *ty == e.ty);
    let fallback = match (other.stmts.is_empty(), other.value.as_deref()) {
        (true, Some(b)) if b.ty == e.ty && simple(b) => b.clone(),
        _ => return None,
    };
    (present && inside).then(|| (value.clone(), fallback))
}

/// A fallback small and pure enough to copy into each branch.
fn simple(b: &Expr) -> bool {
    match &b.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Local(_)
        | ExprKind::None => true,
        ExprKind::Str(s) => s.len() <= 64,
        ExprKind::Record { fields, .. } => fields.is_empty(),
        ExprKind::List(items) => items.is_empty(),
        _ => false,
    }
}

/// Turns the optional `e` into its inside, with `fallback` for `none`, in
/// place. Whether some branch ended in `Some(x)` or `none` (otherwise the
/// rewrite gains nothing).
fn push(e: &mut Expr, fallback: &Expr, ty: Type, locals: &mut Vec<Type>) -> bool {
    match &mut e.kind {
        ExprKind::Some(x) if x.ty == ty => {
            let x = std::mem::replace(&mut **x, hole(e.line));
            *e = x;
            true
        }
        ExprKind::None => {
            *e = fallback.clone();
            true
        }
        ExprKind::Block(b) => {
            let gained = push_block(b, fallback, ty, locals);
            e.ty = ty;
            gained
        }
        ExprKind::If(_, then, other) => {
            let a = push_block(then, fallback, ty, locals);
            let b = push_block(other, fallback, ty, locals);
            e.ty = ty;
            a | b
        }
        // These leave instead of giving a value.
        ExprKind::Panic(_) | ExprKind::Fail(_) => {
            e.ty = ty;
            false
        }
        _ => {
            let value = std::mem::replace(e, hole(e.line));
            restore(e, value, fallback.clone(), locals);
            false
        }
    }
}

fn push_block(b: &mut Block, fallback: &Expr, ty: Type, locals: &mut Vec<Type>) -> bool {
    match b.value.as_deref_mut() {
        Some(v) => push(v, fallback, ty, locals),
        // A block without a value leaves (`return`, `break`).
        None => false,
    }
}

/// A placeholder for an expression being moved.
fn hole(line: u32) -> Expr {
    Expr {
        kind: ExprKind::None,
        ty: Type::Unit,
        line,
    }
}

/// Makes `e` the `??` of `value` and `fallback`, as lowering writes it.
fn restore(e: &mut Expr, value: Expr, fallback: Expr, locals: &mut Vec<Type>) {
    let (ty, line, opt) = (fallback.ty, value.line, value.ty);
    locals.push(opt);
    let tmp = (locals.len() - 1) as Local;
    let read = |kind| Expr { kind, ty, line };
    let local = || {
        Box::new(Expr {
            kind: ExprKind::Local(tmp),
            ty: opt,
            line,
        })
    };
    let present = Expr {
        kind: ExprKind::IsSome(local()),
        ty: Type::Bool,
        line,
    };
    let choice = ExprKind::If(
        Box::new(present),
        Block {
            stmts: Vec::new(),
            value: Some(Box::new(read(ExprKind::Unwrap(local())))),
        },
        Block {
            stmts: Vec::new(),
            value: Some(Box::new(fallback)),
        },
    );
    *e = read(ExprKind::Block(Block {
        stmts: vec![Stmt::Let(tmp, value)],
        value: Some(Box::new(read(choice))),
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPT: Type = Type::Ref(0);

    fn e(kind: ExprKind, ty: Type) -> Expr {
        Expr { kind, ty, line: 1 }
    }

    fn value(e: Expr) -> Block {
        Block {
            stmts: Vec::new(),
            value: Some(Box::new(e)),
        }
    }

    /// `let t = <a>; if t.is_some() { t.unwrap() } else { 0 }`, in a
    /// function whose local 0 is `c: Bool` and local 1 is `t`.
    fn coalesced(a: Expr) -> Function {
        let t = || Box::new(e(ExprKind::Local(1), OPT));
        let choice = ExprKind::If(
            Box::new(e(ExprKind::IsSome(t()), Type::Bool)),
            value(e(ExprKind::Unwrap(t()), Type::I64)),
            value(e(ExprKind::Int(0), Type::I64)),
        );
        let body = ExprKind::Block(Block {
            stmts: vec![Stmt::Let(1, a)],
            value: Some(Box::new(e(choice, Type::I64))),
        });
        Function {
            name: "f".into(),
            params: vec![0],
            ret: Type::I64,
            locals: vec![Type::Bool, OPT],
            body: value(e(body, Type::I64)),
            line: 1,
            fails: false,
            external: None,
        }
    }

    #[test]
    fn some_and_none_branches_give_the_value_and_the_fallback() {
        // `(if c { Some(7) } else { none }) ?? 0` is `if c { 7 } else { 0 }`.
        let some = e(
            ExprKind::Some(Box::new(e(ExprKind::Int(7), Type::I64))),
            OPT,
        );
        let a = ExprKind::If(
            Box::new(e(ExprKind::Local(0), Type::Bool)),
            value(some),
            value(e(ExprKind::None, OPT)),
        );
        let mut f = coalesced(e(a, OPT));
        coalesce(&mut f);
        let body = f.body.value.as_deref().expect("a value");
        let ExprKind::If(_, then, other) = &body.kind else {
            panic!("an if");
        };
        let leaf = |b: &Block| b.value.as_deref().map(|v| v.kind.clone());
        assert!(matches!(leaf(then), Some(ExprKind::Int(7))));
        assert!(matches!(leaf(other), Some(ExprKind::Int(0))));
        assert_eq!(body.ty, Type::I64);
    }

    #[test]
    fn an_optional_from_elsewhere_keeps_its_check() {
        let mut f = coalesced(e(ExprKind::Local(0), OPT));
        coalesce(&mut f);
        let body = f.body.value.as_deref().expect("a value");
        let ExprKind::Block(b) = &body.kind else {
            panic!("the block lowering made");
        };
        assert!(matches!(b.stmts.as_slice(), [Stmt::Let(1, _)]));
    }
}
