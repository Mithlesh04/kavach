//! Looking through KIR: whether any expression in a block matches a test.

use crate::ir::*;

/// Whether `test` holds for any expression in `block`, at any depth.
pub fn any_in_block(block: &Block, test: &mut impl FnMut(&Expr) -> bool) -> bool {
    block.stmts.iter().any(|s| any_in_stmt(s, test))
        || block.value.as_ref().is_some_and(|v| any_expr(v, test))
}

/// Whether running `block` can set local `l`: a `let`, an assignment, a
/// move out of it, or a loop variable.
pub fn sets_local(block: &Block, l: Local) -> bool {
    fn in_stmts(block: &Block, l: Local) -> bool {
        block.stmts.iter().any(|s| match s {
            Stmt::Let(x, _) => *x == l,
            Stmt::LetResult { value, error, .. } => *value == l || *error == l,
            Stmt::ForRange { var, body, .. } => *var == l || in_stmts(body, l),
            Stmt::ForEach {
                var,
                index,
                list,
                body,
            } => *var == l || *index == l || *list == l || in_stmts(body, l),
            Stmt::While { body, .. } => in_stmts(body, l),
            _ => false,
        })
    }
    in_stmts(block, l)
        || any_in_block(block, &mut |e| match &e.kind {
            ExprKind::Assign(x, _) | ExprKind::Move(x) => *x == l,
            ExprKind::If(_, then, otherwise) | ExprKind::Fallback(then, otherwise) => {
                in_stmts(then, l) || in_stmts(otherwise, l)
            }
            ExprKind::Block(b) => in_stmts(b, l),
            _ => false,
        })
}

fn any_in_stmt(s: &Stmt, test: &mut impl FnMut(&Expr) -> bool) -> bool {
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => any_expr(e, test),
        Stmt::LetResult { call, .. } => any_expr(call, test),
        Stmt::Check { cond, .. } => any_expr(cond, test),
        Stmt::While { cond, body } => any_expr(cond, test) || any_in_block(body, test),
        Stmt::ForRange {
            start, end, body, ..
        } => any_expr(start, test) || any_expr(end, test) || any_in_block(body, test),
        Stmt::ForEach { body, .. } => any_in_block(body, test),
        Stmt::Print(parts) => parts.iter().any(|p| match p {
            PrintPart::Value(e) => any_expr(e, test),
            PrintPart::Text(_) => false,
        }),
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => false,
    }
}

/// Whether `test` holds for `e` or any expression inside it.
pub fn any_expr(e: &Expr, test: &mut impl FnMut(&Expr) -> bool) -> bool {
    if test(e) {
        return true;
    }
    match &e.kind {
        ExprKind::Assign(_, x)
        | ExprKind::Unary(_, x)
        | ExprKind::Convert(x, _)
        | ExprKind::Some(x)
        | ExprKind::IsSome(x)
        | ExprKind::Unwrap(x)
        | ExprKind::GetField(x, _)
        | ExprKind::HasVariant(x, _)
        | ExprKind::Parse(x)
        | ExprKind::Fail(x) => any_expr(x, test),
        ExprKind::Binary(_, a, b)
        | ExprKind::Fast(_, a, b)
        | ExprKind::And(a, b)
        | ExprKind::Or(a, b)
        | ExprKind::Index(a, b)
        | ExprKind::InRange(a, b)
        | ExprKind::Context(a, b)
        | ExprKind::SetField(a, _, b) => any_expr(a, test) || any_expr(b, test),
        ExprKind::Format(items)
        | ExprKind::List(items)
        | ExprKind::Tuple(items)
        | ExprKind::Call(_, items)
        | ExprKind::Rt(_, items)
        | ExprKind::Record { fields: items, .. }
        | ExprKind::Closure {
            captures: items, ..
        } => items.iter().any(|x| any_expr(x, test)),
        ExprKind::CallValue(callee, args) => {
            any_expr(callee, test) || args.iter().any(|x| any_expr(x, test))
        }
        ExprKind::Map(pairs) => pairs
            .iter()
            .any(|(k, v)| any_expr(k, test) || any_expr(v, test)),
        ExprKind::If(cond, then, otherwise) => {
            any_expr(cond, test) || any_in_block(then, test) || any_in_block(otherwise, test)
        }
        ExprKind::Block(b) => any_in_block(b, test),
        ExprKind::Fallback(fast, slow) => any_in_block(fast, test) || any_in_block(slow, test),
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Bool(_)
        | ExprKind::Char(_)
        | ExprKind::Str(_)
        | ExprKind::Decimal(_)
        | ExprKind::Local(_)
        | ExprKind::Move(_)
        | ExprKind::Panic(_)
        | ExprKind::None => false,
    }
}

/// Calls `f` on every expression in `block`, at any depth, children
/// before their parent (so `f` may replace an expression without seeing
/// the replacement).
pub fn exprs_mut(block: &mut Block, f: &mut impl FnMut(&mut Expr)) {
    for s in &mut block.stmts {
        stmt_exprs_mut(s, f);
    }
    if let Some(v) = &mut block.value {
        expr_mut(v, f);
    }
}

fn stmt_exprs_mut(s: &mut Stmt, f: &mut impl FnMut(&mut Expr)) {
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => expr_mut(e, f),
        Stmt::LetResult { call, .. } => expr_mut(call, f),
        Stmt::Check { cond, .. } => expr_mut(cond, f),
        Stmt::While { cond, body } => {
            expr_mut(cond, f);
            exprs_mut(body, f);
        }
        Stmt::ForRange {
            start, end, body, ..
        } => {
            expr_mut(start, f);
            expr_mut(end, f);
            exprs_mut(body, f);
        }
        Stmt::ForEach { body, .. } => exprs_mut(body, f),
        Stmt::Print(parts) => {
            for p in parts {
                if let PrintPart::Value(e) = p {
                    expr_mut(e, f);
                }
            }
        }
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
    }
}

fn expr_mut(e: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    parts(e, &mut |p| match p {
        Part::Expr(x) => expr_mut(x, f),
        Part::Block(b) => exprs_mut(b, f),
    });
    f(e);
}

/// A direct part of an expression: a smaller expression, or a block.
pub(crate) enum Part<'a> {
    Expr(&'a mut Expr),
    Block(&'a mut Block),
}

/// Calls `visit` on each direct part of statement `s`, in order.
pub(crate) fn stmt_parts(s: &mut Stmt, visit: &mut impl FnMut(Part<'_>)) {
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => visit(Part::Expr(e)),
        Stmt::LetResult { call, .. } => visit(Part::Expr(call)),
        Stmt::Check { cond, .. } => visit(Part::Expr(cond)),
        Stmt::While { cond, body } => {
            visit(Part::Expr(cond));
            visit(Part::Block(body));
        }
        Stmt::ForRange {
            start, end, body, ..
        } => {
            visit(Part::Expr(start));
            visit(Part::Expr(end));
            visit(Part::Block(body));
        }
        Stmt::ForEach { body, .. } => visit(Part::Block(body)),
        Stmt::Print(ps) => {
            for p in ps {
                if let PrintPart::Value(e) = p {
                    visit(Part::Expr(e));
                }
            }
        }
        Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
    }
}

/// Calls `visit` on each direct part of `e`, in order.
pub(crate) fn parts(e: &mut Expr, visit: &mut impl FnMut(Part<'_>)) {
    match &mut e.kind {
        ExprKind::Assign(_, x)
        | ExprKind::Unary(_, x)
        | ExprKind::Convert(x, _)
        | ExprKind::Some(x)
        | ExprKind::IsSome(x)
        | ExprKind::Unwrap(x)
        | ExprKind::GetField(x, _)
        | ExprKind::HasVariant(x, _)
        | ExprKind::Parse(x)
        | ExprKind::Fail(x) => visit(Part::Expr(x)),
        ExprKind::Binary(_, a, b)
        | ExprKind::Fast(_, a, b)
        | ExprKind::And(a, b)
        | ExprKind::Or(a, b)
        | ExprKind::Index(a, b)
        | ExprKind::InRange(a, b)
        | ExprKind::Context(a, b)
        | ExprKind::SetField(a, _, b) => {
            visit(Part::Expr(a));
            visit(Part::Expr(b));
        }
        ExprKind::Format(items)
        | ExprKind::List(items)
        | ExprKind::Tuple(items)
        | ExprKind::Call(_, items)
        | ExprKind::Rt(_, items)
        | ExprKind::Record { fields: items, .. }
        | ExprKind::Closure {
            captures: items, ..
        } => items.iter_mut().for_each(|x| visit(Part::Expr(x))),
        ExprKind::CallValue(callee, args) => {
            visit(Part::Expr(callee));
            args.iter_mut().for_each(|x| visit(Part::Expr(x)));
        }
        ExprKind::Map(pairs) => {
            for (k, v) in pairs {
                visit(Part::Expr(k));
                visit(Part::Expr(v));
            }
        }
        ExprKind::If(cond, then, otherwise) => {
            visit(Part::Expr(cond));
            visit(Part::Block(then));
            visit(Part::Block(otherwise));
        }
        ExprKind::Block(b) => visit(Part::Block(b)),
        ExprKind::Fallback(fast, slow) => {
            visit(Part::Block(fast));
            visit(Part::Block(slow));
        }
        _ => {}
    }
}

/// Calls `f` on `block` and on every block in it, at any depth (loop
/// bodies, and the blocks of `if`, blocks and fallbacks), inner ones
/// first.
pub fn blocks_mut(block: &mut Block, f: &mut impl FnMut(&mut Block)) {
    for s in &mut block.stmts {
        match s {
            Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => expr_blocks(e, f),
            Stmt::LetResult { call, .. } => expr_blocks(call, f),
            Stmt::Check { cond, .. } => expr_blocks(cond, f),
            Stmt::While { cond, body } => {
                expr_blocks(cond, f);
                blocks_mut(body, f);
            }
            Stmt::ForRange {
                start, end, body, ..
            } => {
                expr_blocks(start, f);
                expr_blocks(end, f);
                blocks_mut(body, f);
            }
            Stmt::ForEach { body, .. } => blocks_mut(body, f),
            Stmt::Print(ps) => {
                for p in ps {
                    if let PrintPart::Value(e) = p {
                        expr_blocks(e, f);
                    }
                }
            }
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
        }
    }
    if let Some(v) = &mut block.value {
        expr_blocks(v, f);
    }
    f(block);
}

fn expr_blocks(e: &mut Expr, f: &mut impl FnMut(&mut Block)) {
    parts(e, &mut |p| match p {
        Part::Expr(x) => expr_blocks(x, f),
        Part::Block(b) => blocks_mut(b, f),
    });
}

/// Calls `f` on every mention of a local in `block`: in statements that
/// set one and in expressions that read, move or assign one.
pub fn locals_mut(block: &mut Block, f: &mut impl FnMut(&mut Local)) {
    fn stmts(block: &mut Block, f: &mut impl FnMut(&mut Local)) {
        for s in &mut block.stmts {
            match s {
                Stmt::Let(l, _) => f(l),
                Stmt::LetResult { value, error, .. } => {
                    f(value);
                    f(error);
                }
                Stmt::ForRange { var, body, .. } => {
                    f(var);
                    stmts(body, f);
                }
                Stmt::ForEach {
                    var,
                    index,
                    list,
                    body,
                } => {
                    f(var);
                    f(index);
                    f(list);
                    stmts(body, f);
                }
                Stmt::While { body, .. } => stmts(body, f),
                _ => {}
            }
        }
    }
    stmts(block, f);
    exprs_mut(block, &mut |e| match &mut e.kind {
        ExprKind::Local(l) | ExprKind::Move(l) | ExprKind::Assign(l, _) => f(l),
        ExprKind::If(_, then, otherwise) | ExprKind::Fallback(then, otherwise) => {
            stmts(then, f);
            stmts(otherwise, f);
        }
        ExprKind::Block(b) => stmts(b, f),
        _ => {}
    });
}
