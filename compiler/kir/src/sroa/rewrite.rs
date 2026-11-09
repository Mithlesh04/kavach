//! The rewrite half of scalar replacement: once `scalarize` has
//! chosen the locals to split, every use of them is rewritten to the
//! per-field locals. See the module doc in `sroa.rs`.

use super::{as_local, as_store};
use crate::ir::*;
use std::collections::HashMap;

/// Blocks nested inside an expression (`if` arms, block values).
fn each_block_mut(e: &mut Expr, f: &mut impl FnMut(&mut Block)) {
    match &mut e.kind {
        ExprKind::If(c, a, b) => {
            each_block_mut(c, f);
            f(a);
            f(b);
        }
        ExprKind::Fallback(a, b) => {
            f(a);
            f(b);
        }
        ExprKind::Block(b) => f(b),
        _ => each_child_mut(e, &mut |x| each_block_mut(x, f)),
    }
}

fn each_child_mut(e: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    match &mut e.kind {
        ExprKind::Call(_, xs)
        | ExprKind::Rt(_, xs)
        | ExprKind::List(xs)
        | ExprKind::Tuple(xs)
        | ExprKind::Record { fields: xs, .. }
        | ExprKind::Format(xs) => xs.iter_mut().for_each(f),
        ExprKind::CallValue(x, xs) => {
            f(x);
            xs.iter_mut().for_each(f);
        }
        ExprKind::Closure { captures, .. } => captures.iter_mut().for_each(f),
        ExprKind::Map(pairs) => pairs.iter_mut().for_each(|(k, v)| {
            f(k);
            f(v);
        }),
        ExprKind::Assign(_, x)
        | ExprKind::Unary(_, x)
        | ExprKind::Some(x)
        | ExprKind::IsSome(x)
        | ExprKind::Unwrap(x)
        | ExprKind::Convert(x, _)
        | ExprKind::Fail(x)
        | ExprKind::GetField(x, _)
        | ExprKind::Parse(x)
        | ExprKind::HasVariant(x, _) => f(x),
        ExprKind::Binary(_, a, b)
        | ExprKind::Fast(_, a, b)
        | ExprKind::And(a, b)
        | ExprKind::Or(a, b)
        | ExprKind::Index(a, b)
        | ExprKind::InRange(a, b)
        | ExprKind::Context(a, b) => {
            f(a);
            f(b);
        }
        ExprKind::SetField(a, _, b) => {
            f(a);
            f(b);
        }
        ExprKind::If(c, _, _) => f(c),
        _ => {}
    }
}

/// What a split local becomes: its per-element locals, and whether it
/// was a list (rebuilt as a list literal) or a record.
pub(super) struct Parts {
    pub(super) news: Vec<Local>,
    pub(super) list: bool,
}

pub(super) type Split = HashMap<Local, Parts>;

pub(super) fn rewrite_block(b: &mut Block, split: &Split, locals: &mut Vec<Type>) {
    for s in std::mem::take(&mut b.stmts) {
        rewrite_stmt(s, split, locals, &mut b.stmts);
    }
    if let Some(v) = &mut b.value {
        rewrite_expr(v, split, locals);
    }
}

fn rewrite_stmt(mut s: Stmt, split: &Split, locals: &mut Vec<Type>, out: &mut Vec<Stmt>) {
    if let Some((l, _)) = as_store(&s)
        && split.contains_key(&l)
    {
        let value = match s {
            Stmt::Let(_, v) => v,
            Stmt::Expr(e) => match e.kind {
                ExprKind::Assign(_, v) => *v,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };
        store(&split[&l].news, value, split, locals, out);
        return;
    }
    match &mut s {
        Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => {
            rewrite_expr(e, split, locals);
        }
        Stmt::LetResult { call, .. } => rewrite_expr(call, split, locals),
        Stmt::While { cond, body } => {
            rewrite_expr(cond, split, locals);
            rewrite_block(body, split, locals);
        }
        Stmt::ForRange {
            start, end, body, ..
        } => {
            rewrite_expr(start, split, locals);
            rewrite_expr(end, split, locals);
            rewrite_block(body, split, locals);
        }
        Stmt::ForEach { body, .. } => rewrite_block(body, split, locals),
        Stmt::Check { cond, .. } => rewrite_expr(cond, split, locals),
        Stmt::Print(parts) => {
            for p in parts {
                if let PrintPart::Value(e) = p {
                    rewrite_expr(e, split, locals);
                }
            }
        }
        _ => {}
    }
    out.push(s);
}

/// The split fields of `news` take `value`, field by field.
fn store(news: &[Local], value: Expr, split: &Split, locals: &mut Vec<Type>, out: &mut Vec<Stmt>) {
    let line = value.line;
    match value.kind {
        ExprKind::Record {
            variant: 0,
            mut fields,
        } => {
            // Field expressions run in order into temporaries first, in
            // case a later one reads a field being replaced.
            let mut temps = Vec::new();
            for x in &mut fields {
                rewrite_expr(x, split, locals);
                let t = locals.len() as Local;
                locals.push(x.ty);
                out.push(Stmt::Let(t, x.clone()));
                temps.push((t, x.ty));
            }
            for (l, (t, ty)) in news.iter().zip(temps) {
                out.push(assign(*l, read(t, ty, line)));
            }
        }
        ExprKind::Local(o) | ExprKind::Move(o) => {
            let from = &split[&o].news;
            for (l, src) in news.iter().zip(from) {
                out.push(assign(*l, read(*src, locals[*src as usize], line)));
            }
        }
        ExprKind::List(mut items) => {
            let mut temps = Vec::new();
            for x in &mut items {
                rewrite_expr(x, split, locals);
                let t = locals.len() as Local;
                locals.push(x.ty);
                out.push(Stmt::Let(t, x.clone()));
                temps.push((t, x.ty));
            }
            for (l, (t, ty)) in news.iter().zip(temps) {
                out.push(assign(*l, read(t, ty, line)));
            }
        }
        ExprKind::Block(mut b) => {
            for s in std::mem::take(&mut b.stmts) {
                rewrite_stmt(s, split, locals, out);
            }
            let v = *b.value.expect("a judged store");
            store(news, v, split, locals, out);
        }
        ExprKind::SetField(base, i, mut v) => {
            rewrite_expr(&mut v, split, locals);
            let t = locals.len() as Local;
            locals.push(v.ty);
            out.push(Stmt::Let(t, *v));
            let o = as_local(&base).expect("a judged store");
            let from = split[&o].news.clone();
            for (k, (l, src)) in news.iter().zip(&from).enumerate() {
                let field = if k as u32 == i {
                    read(t, locals[t as usize], line)
                } else {
                    read(*src, locals[*src as usize], line)
                };
                out.push(assign(*l, field));
            }
        }
        _ => unreachable!("a judged store"),
    }
}

fn read(l: Local, ty: Type, line: u32) -> Expr {
    Expr {
        kind: ExprKind::Local(l),
        ty,
        line,
    }
}

fn assign(l: Local, value: Expr) -> Stmt {
    let line = value.line;
    Stmt::Expr(Expr {
        kind: ExprKind::Assign(l, Box::new(value)),
        ty: Type::Unit,
        line,
    })
}

fn rewrite_expr(e: &mut Expr, split: &Split, locals: &mut Vec<Type>) {
    // `q.x` of a split record first, so the base local isn't rebuilt.
    if let ExprKind::GetField(base, i) = &e.kind
        && let Some(o) = as_local(base)
        && let Some(parts) = split.get(&o)
    {
        let l = parts.news[*i as usize];
        e.kind = ExprKind::Local(l);
        return;
    }
    // `xs[2]` of a split list, the same way.
    if let ExprKind::Index(base, idx) = &e.kind
        && let (Some(o), ExprKind::Int(k)) = (as_local(base), &idx.kind)
        && let Some(parts) = split.get(&o)
    {
        let l = parts.news[*k as usize];
        e.kind = ExprKind::Local(l);
        return;
    }
    // An assignment into a split local, used as an expression.
    if let ExprKind::Assign(l, _) = &e.kind
        && split.contains_key(l)
    {
        let (l, value) = match std::mem::replace(&mut e.kind, ExprKind::None) {
            ExprKind::Assign(l, v) => (l, *v),
            _ => unreachable!(),
        };
        let mut stmts = Vec::new();
        store(&split[&l].news, value, split, locals, &mut stmts);
        e.kind = ExprKind::Block(Block { stmts, value: None });
        return;
    }
    // A split value flowing somewhere whole: rebuild it there.
    if let Some(o) = as_local(e)
        && let Some(parts) = split.get(&o)
    {
        let fields: Vec<Expr> = parts
            .news
            .iter()
            .map(|l| read(*l, locals[*l as usize], e.line))
            .collect();
        e.kind = if parts.list {
            ExprKind::List(fields)
        } else {
            ExprKind::Record { variant: 0, fields }
        };
        return;
    }
    each_child_mut(e, &mut |x| rewrite_expr(x, split, locals));
    each_block_mut(e, &mut |b| rewrite_block(b, split, locals));
}
