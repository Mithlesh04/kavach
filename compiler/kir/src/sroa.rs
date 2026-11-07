//! Scalar replacement: a record local the function never shares is split
//! into one local per field, so copying it, changing a field and reading
//! one are plain moves between locals — no allocation, no reference
//! count, and the registers C++ would use. A record of three integers
//! copied in a loop cost an allocation per round before this.
//!
//! Only records whose fields are all numbers, booleans or characters are
//! split, and only locals whose every use the pass understands: built
//! (`Record`), copied to or from another local, changed one field at a
//! time (`SetField`), or read a field at a time (`GetField`). Copying a
//! split local into anything kept whole rebuilds the record there; a
//! local used any other way — passed on, returned, stored inside
//! something — is kept whole. Unknown always means keep.
//!
//! A list local works the same when every store to it is a list literal
//! of one fixed length and every read is `xs[k]` with a constant `k` in
//! range: the short-lived list a loop builds and reads never exists.

use crate::ir::*;
use rewrite::{Split, rewrite_block};

mod rewrite;
#[cfg(test)]
mod tests;

pub fn scalarize(f: &mut Function, shapes: &[Shape], holders: &std::collections::HashSet<Type>) {
    if std::env::var_os("KAVACH_NO_SROA").is_some() {
        return;
    }
    if let Ok(only) = std::env::var("KAVACH_SROA_ONLY")
        && !only.split(',').any(|n| n == f.name)
    {
        return;
    }
    let field_types = |l: Local| -> Option<Vec<Type>> {
        let Type::Ref(id) = f.locals[l as usize] else {
            return None;
        };
        // A `Drop` value must be an object, freed when its holder ends.
        if holders.contains(&Type::Ref(id)) {
            return None;
        }
        let Shape::Record { fields, .. } = &shapes[id as usize] else {
            return None;
        };
        fields
            .iter()
            .all(|x| !x.ty.is_ref() && x.ty != Type::Unit)
            .then(|| fields.iter().map(|x| x.ty).collect())
    };
    let mut candidates: Vec<Option<Vec<Type>>> = (0..f.locals.len())
        .map(|l| field_types(l as Local))
        .collect();
    let mut is_list = vec![false; f.locals.len()];
    let mut lens: Vec<Option<usize>> = vec![None; f.locals.len()];
    list_lens(&f.body, f, shapes, &mut lens);
    for (l, len) in lens.iter().enumerate() {
        if let (Some(n), Type::Ref(id)) = (len, f.locals[l])
            && let Shape::List(t) = &shapes[id as usize]
        {
            candidates[l] = Some(vec![*t; *n]);
            is_list[l] = true;
        }
    }
    if candidates.iter().all(Option::is_none) {
        return;
    }
    // Keeping one local whole can keep another (a copy between them), so
    // scan until nothing more changes.
    let mut keep: Vec<bool> = candidates.iter().map(Option::is_none).collect();
    // A parameter is the signature's: callers fill it whole.
    for p in &f.params {
        keep[*p as usize] = true;
    }
    loop {
        let before = keep.clone();
        let mut found = Vec::new();
        scan_block(&f.body, &before, &lens, &mut |l| found.push(l));
        let mut changed = false;
        for l in found {
            if !keep[l as usize] {
                keep[l as usize] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut locals = std::mem::take(&mut f.locals);
    let split: Split = candidates
        .iter()
        .enumerate()
        .filter(|(l, _)| !keep[*l])
        .map(|(l, c)| {
            let tys = c.as_ref().expect("a candidate");
            let news: Vec<Local> = tys
                .iter()
                .map(|t| {
                    locals.push(*t);
                    (locals.len() - 1) as Local
                })
                .collect();
            (
                l as Local,
                rewrite::Parts {
                    news,
                    list: is_list[l],
                },
            )
        })
        .collect();
    if split.is_empty() {
        f.locals = locals;
        return;
    }
    if std::env::var_os("KAVACH_SROA_LOG").is_some() {
        eprintln!(
            "sroa: {} splits {:?}",
            f.name,
            split.keys().collect::<Vec<_>>()
        );
    }
    rewrite_block(&mut f.body, &split, &mut locals);
    if std::env::var_os("KAVACH_SROA_LOG").is_some() {
        crate::walk::any_in_block(&f.body, &mut |e| {
            let l = match e.kind {
                ExprKind::Local(l) | ExprKind::Move(l) => Some(l),
                ExprKind::Assign(l, _) => Some(l),
                ExprKind::GetField(ref b, _) => as_local(b),
                _ => None,
            };
            if let Some(l) = l
                && split.contains_key(&l)
            {
                eprintln!(
                    "sroa LEFTOVER in {}: local {l} in {:?} line {}",
                    f.name,
                    kind_name(&e.kind),
                    e.line
                );
            }
            false
        });
    }
    f.locals = locals;
}

pub(super) fn kind_name(k: &ExprKind) -> &'static str {
    match k {
        ExprKind::Local(_) => "Local",
        ExprKind::Move(_) => "Move",
        ExprKind::Assign(..) => "Assign",
        ExprKind::GetField(..) => "GetField",
        _ => "other",
    }
}

/// Sizes every list local: `Some(n)` when every store to it is a list
/// literal of `n` scalar items (n at most 16); `None` otherwise.
fn list_lens(b: &Block, f: &Function, shapes: &[Shape], lens: &mut [Option<usize>]) {
    let mut disqualified = vec![false; f.locals.len()];
    size_block(b, &mut |l, value| {
        let scalar_list = matches!(f.locals[l as usize], Type::Ref(id)
            if matches!(&shapes[id as usize], Shape::List(t) if !t.is_ref() && *t != Type::Unit));
        if !scalar_list || disqualified[l as usize] {
            return;
        }
        let fits = match literal_len(value) {
            Some(n) if (1..=16).contains(&n) => match lens[l as usize] {
                Some(seen) => seen == n,
                None => {
                    lens[l as usize] = Some(n);
                    true
                }
            },
            _ => false,
        };
        if !fits {
            disqualified[l as usize] = true;
            lens[l as usize] = None;
        }
    });
    for p in &f.params {
        lens[*p as usize] = None;
    }
}

/// The length of a list-literal store, looking through an inlined
/// call's block to its value.
fn literal_len(value: &Expr) -> Option<usize> {
    match &value.kind {
        ExprKind::List(items) => Some(items.len()),
        ExprKind::Block(b) => literal_len(b.value.as_deref()?),
        _ => None,
    }
}

/// Calls `f` on every store, at any depth, each exactly once.
fn size_block(b: &Block, f: &mut impl FnMut(Local, &Expr)) {
    for s in &b.stmts {
        if let Some((l, value)) = as_store(s) {
            f(l, value);
        }
        match s {
            Stmt::Let(_, e) | Stmt::Expr(e) | Stmt::Return(Some(e)) => size_expr(e, f),
            Stmt::LetResult { call, .. } => size_expr(call, f),
            Stmt::While { cond, body } => {
                size_expr(cond, f);
                size_block(body, f);
            }
            Stmt::ForRange {
                start, end, body, ..
            } => {
                size_expr(start, f);
                size_expr(end, f);
                size_block(body, f);
            }
            Stmt::ForEach { body, .. } => size_block(body, f),
            Stmt::Check { cond, .. } => size_expr(cond, f),
            Stmt::Print(parts) => {
                for p in parts {
                    if let PrintPart::Value(e) = p {
                        size_expr(e, f);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(v) = &b.value {
        size_expr(v, f);
    }
}

fn size_expr(e: &Expr, f: &mut impl FnMut(Local, &Expr)) {
    match &e.kind {
        ExprKind::Assign(l, v) => {
            f(*l, v);
            size_expr(v, f);
        }
        ExprKind::If(c, a, b) => {
            size_expr(c, f);
            size_block(a, f);
            size_block(b, f);
        }
        ExprKind::Fallback(a, b) => {
            size_block(a, f);
            size_block(b, f);
        }
        ExprKind::Block(b) => size_block(b, f),
        _ => each_child(e, &mut |x| size_expr(x, f)),
    }
}

/// A store's target and value, in either spelling.
pub(super) fn as_store(s: &Stmt) -> Option<(Local, &Expr)> {
    match s {
        Stmt::Let(l, v) => Some((*l, v)),
        Stmt::Expr(Expr {
            kind: ExprKind::Assign(l, v),
            ..
        }) => Some((*l, v)),
        _ => None,
    }
}

pub(super) fn as_local(e: &Expr) -> Option<Local> {
    match e.kind {
        ExprKind::Local(l) | ExprKind::Move(l) => Some(l),
        _ => None,
    }
}

/// Walks `b` reporting every candidate that must stay whole. `keep[l]`
/// is what is already kept; a copy between a split and a kept local
/// keeps both sides consistent.
fn scan_block(b: &Block, keep: &[bool], lens: &[Option<usize>], out: &mut impl FnMut(Local)) {
    scan_stmts(b, keep, lens, out);
    if let Some(v) = &b.value {
        scan_expr(v, keep, lens, out);
    }
}

fn scan_stmts(b: &Block, keep: &[bool], lens: &[Option<usize>], out: &mut impl FnMut(Local)) {
    for s in &b.stmts {
        if let Some((l, value)) = as_store(s) {
            scan_store(l, value, keep, lens, out);
            continue;
        }
        match s {
            Stmt::Expr(e) | Stmt::Return(Some(e)) => scan_expr(e, keep, lens, out),
            // A fallible call fills its result whole.
            Stmt::LetResult { value, error, call } => {
                out(*value);
                out(*error);
                scan_expr(call, keep, lens, out);
            }
            Stmt::While { cond, body } => {
                scan_expr(cond, keep, lens, out);
                scan_block(body, keep, lens, out);
            }
            Stmt::ForRange {
                start, end, body, ..
            } => {
                scan_expr(start, keep, lens, out);
                scan_expr(end, keep, lens, out);
                scan_block(body, keep, lens, out);
            }
            Stmt::ForEach {
                var, list, body, ..
            } => {
                out(*var);
                out(*list);
                scan_block(body, keep, lens, out);
            }
            Stmt::Check { cond, .. } => scan_expr(cond, keep, lens, out),
            Stmt::Print(parts) => {
                for p in parts {
                    if let PrintPart::Value(e) = p {
                        scan_expr(e, keep, lens, out);
                    }
                }
            }
            _ => {}
        }
    }
}

/// `l = value`: the forms the pass can split, or both sides kept.
fn scan_store(
    l: Local,
    value: &Expr,
    keep: &[bool],
    lens: &[Option<usize>],
    out: &mut impl FnMut(Local),
) {
    match &value.kind {
        ExprKind::Record { variant: 0, fields } => {
            for x in fields {
                scan_expr(x, keep, lens, out);
            }
        }
        // A list literal: splittable like a record build (the pre-scan
        // made sure every store to a candidate has the same length).
        ExprKind::List(items) => {
            for x in items {
                scan_expr(x, keep, lens, out);
            }
        }
        ExprKind::Local(o) | ExprKind::Move(o) if value.ty.is_ref() => {
            // A copy: a kept source keeps the target (its value came from
            // somewhere whole); a kept target rebuilds from a split
            // source, which is fine.
            if keep[*o as usize] {
                out(l);
            }
        }
        ExprKind::SetField(base, _, v) => {
            scan_expr(v, keep, lens, out);
            match as_local(base) {
                // A whole base makes a whole copy: the target stays whole
                // too (the rewrite can't explode it from a whole record).
                Some(o) if keep[o as usize] => out(l),
                Some(_) => {}
                None => {
                    scan_expr(base, keep, lens, out);
                    out(l);
                }
            }
        }
        // An inlined call: the block's statements are ordinary code, and
        // its value is what is stored.
        ExprKind::Block(b) => {
            scan_stmts(b, keep, lens, out);
            match &b.value {
                Some(v) => scan_store(l, v, keep, lens, out),
                None => out(l),
            }
        }
        _ => {
            scan_expr(value, keep, lens, out);
            out(l);
        }
    }
}

/// Any whole-record read outside the store forms keeps the local, except
/// `x.field`, which splits.
fn scan_expr(e: &Expr, keep: &[bool], lens: &[Option<usize>], out: &mut impl FnMut(Local)) {
    match &e.kind {
        ExprKind::Local(l) | ExprKind::Move(l) => out(*l),
        ExprKind::GetField(base, _) if as_local(base).is_some() => {}
        // `xs[2]` with a constant index in range reads one element.
        ExprKind::Index(base, idx) => match (as_local(base), &idx.kind) {
            (Some(o), ExprKind::Int(k))
                if lens[o as usize].is_some_and(|n| (0..n as i128).contains(k)) => {}
            _ => {
                scan_expr(base, keep, lens, out);
                scan_expr(idx, keep, lens, out);
            }
        },
        ExprKind::Assign(l, v) => scan_store(*l, v, keep, lens, out),
        ExprKind::If(c, a, b) => {
            scan_expr(c, keep, lens, out);
            scan_block(a, keep, lens, out);
            scan_block(b, keep, lens, out);
        }
        ExprKind::Fallback(a, b) => {
            scan_block(a, keep, lens, out);
            scan_block(b, keep, lens, out);
        }
        ExprKind::Block(b) => scan_block(b, keep, lens, out),
        _ => each_child(e, &mut |x| scan_expr(x, keep, lens, out)),
    }
}

/// Calls `f` on every direct child expression of `e`.
fn each_child<'e>(e: &'e Expr, f: &mut impl FnMut(&'e Expr)) {
    match &e.kind {
        ExprKind::Call(_, xs)
        | ExprKind::Rt(_, xs)
        | ExprKind::List(xs)
        | ExprKind::Tuple(xs)
        | ExprKind::Record { fields: xs, .. }
        | ExprKind::Format(xs) => xs.iter().for_each(f),
        ExprKind::CallValue(x, xs) => {
            f(x);
            xs.iter().for_each(f);
        }
        ExprKind::Closure { captures, .. } => captures.iter().for_each(f),
        ExprKind::Map(pairs) => pairs.iter().for_each(|(k, v)| {
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
