//! Which loops may go on all cores (decision 0031), and what they
//! capture: the analysis behind `par.rs`. A loop qualifies when its
//! body only does arithmetic, reads lists it doesn't write, and stores
//! `out[i]` into lists the function made and still owns alone.

use crate::ir::{BinOp, Block, Expr, ExprKind, FastOp, Local, Stmt, Type};
use crate::rt_op::RtOp;

/// `kv_par_for` passes captures as plain `i64` arguments.
const MAX_CAPS: usize = 6;

/// A list stops being "fresh" when anything but a self `push`/`set` or a
/// read touches it: passed to a call, stored, copied, printed.
pub(super) fn unshare_stmt(s: &Stmt, fresh: &mut Vec<Local>) {
    match s {
        Stmt::Let(_, e) | Stmt::Expr(e) => unshare(e, fresh),
        Stmt::Check { cond, .. } => unshare(cond, fresh),
        Stmt::Return(Some(e)) => unshare(e, fresh),
        Stmt::ForRange {
            start, end, body, ..
        } => {
            unshare(start, fresh);
            unshare(end, fresh);
            for s in &body.stmts {
                unshare_stmt(s, fresh);
            }
        }
        Stmt::While { cond, body } => {
            unshare(cond, fresh);
            for s in &body.stmts {
                unshare_stmt(s, fresh);
            }
        }
        Stmt::ForEach { list, body, .. } => {
            fresh.retain(|x| x != list);
            for s in &body.stmts {
                unshare_stmt(s, fresh);
            }
        }
        Stmt::LetResult { call, .. } => unshare(call, fresh),
        Stmt::Print(parts) => {
            for p in parts {
                if let crate::ir::PrintPart::Value(e) = p {
                    unshare(e, fresh);
                }
            }
        }
        _ => {}
    }
}

/// Removes from `fresh` every list `e` could share, keeping the two safe
/// self-shapes: `out = out.push(x)` / `out[i] = v` reassignments, and
/// `list[i]` reads.
pub(super) fn unshare(e: &Expr, fresh: &mut Vec<Local>) {
    match &e.kind {
        ExprKind::Assign(l, v) => {
            // `out = push(move out, x)` and `out = set(move out, i, v)`
            // keep `out` the only owner.
            if let ExprKind::Rt(RtOp::ListPush | RtOp::ListSet, args) = &v.kind
                && matches!(&args[0].kind, ExprKind::Move(m) | ExprKind::Local(m) if m == l)
            {
                for a in &args[1..] {
                    unshare(a, fresh);
                }
                return;
            }
            unshare(v, fresh);
            fresh.retain(|x| x != l);
        }
        ExprKind::Index(base, i) => {
            // A read borrows; the list stays this function's own.
            if !matches!(base.kind, ExprKind::Local(_)) {
                unshare(base, fresh);
            }
            unshare(i, fresh);
        }
        ExprKind::Rt(RtOp::ListLen, args) if matches!(args[0].kind, ExprKind::Local(_)) => {}
        ExprKind::Local(l) | ExprKind::Move(l) => fresh.retain(|x| x != l),
        ExprKind::If(cond, then, otherwise) => {
            unshare(cond, fresh);
            unshare_block(then, fresh);
            unshare_block(otherwise, fresh);
        }
        ExprKind::Block(b) => unshare_block(b, fresh),
        ExprKind::Fallback(fast, slow) => {
            unshare_block(fast, fresh);
            unshare_block(slow, fresh);
        }
        _ => {
            for part in parts_of(e) {
                unshare(part, fresh);
            }
        }
    }
}

fn unshare_block(b: &Block, fresh: &mut Vec<Local>) {
    for s in &b.stmts {
        unshare_stmt(s, fresh);
    }
    if let Some(v) = &b.value {
        unshare(v, fresh);
    }
}

/// The direct subexpressions of `e`.
fn parts_of(e: &Expr) -> Vec<&Expr> {
    match &e.kind {
        ExprKind::Binary(_, a, b)
        | ExprKind::Fast(_, a, b)
        | ExprKind::Index(a, b)
        | ExprKind::InRange(a, b)
        | ExprKind::Context(a, b)
        | ExprKind::SetField(a, _, b)
        | ExprKind::And(a, b)
        | ExprKind::Or(a, b) => vec![a, b],
        ExprKind::Unary(_, x)
        | ExprKind::Convert(x, _)
        | ExprKind::Some(x)
        | ExprKind::IsSome(x)
        | ExprKind::Unwrap(x)
        | ExprKind::GetField(x, _)
        | ExprKind::HasVariant(x, _)
        | ExprKind::Parse(x)
        | ExprKind::Fail(x)
        | ExprKind::Assign(_, x) => vec![x],
        ExprKind::Rt(_, args)
        | ExprKind::Call(_, args)
        | ExprKind::List(args)
        | ExprKind::Tuple(args) => args.iter().collect(),
        ExprKind::CallValue(f, args) => {
            let mut v = vec![&**f];
            v.extend(args.iter());
            v
        }
        _ => Vec::new(),
    }
}

/// What an eligible loop needs from outside: the locals it reads (all
/// `i64` slots — integers and lists), in first-use order.
pub(super) struct Plan {
    pub(super) caps: Vec<Local>,
}

/// Whether this loop body does nothing but arithmetic, reads of lists it
/// doesn't write, and `out[i] = value` stores into fresh lists at the
/// loop's own index. Gives the captures when it does.
pub(super) fn eligible(body: &Block, var: Local, fresh: &[Local], locals: &[Type]) -> Option<Plan> {
    if body.value.is_some() {
        return None;
    }
    let mut outs = Vec::new();
    let mut temps = vec![var];
    let mut caps = Vec::new();
    // First find the written lists: reads of them are races.
    for s in &body.stmts {
        if let Stmt::Expr(e) = s
            && let ExprKind::Assign(out, v) = &e.kind
            && let ExprKind::Rt(RtOp::ListSet, _) = &v.kind
        {
            outs.push(*out);
        }
    }
    for s in &body.stmts {
        match s {
            Stmt::Let(l, e) => {
                if e.ty.is_ref() || !value_ok(e, &temps, &outs, fresh, locals, &mut caps) {
                    return None;
                }
                temps.push(*l);
            }
            Stmt::Expr(e) => {
                let ExprKind::Assign(out, v) = &e.kind else {
                    return None;
                };
                let ExprKind::Rt(RtOp::ListSet, args) = &v.kind else {
                    return None;
                };
                if !fresh.contains(out)
                    || !matches!(&args[0].kind, ExprKind::Move(m) | ExprKind::Local(m) if m == out)
                    || !matches!(&args[1].kind, ExprKind::Local(i) if *i == var)
                    || args[2].ty.is_ref()
                    || !value_ok(&args[2], &temps, &outs, fresh, locals, &mut caps)
                {
                    return None;
                }
                capture(*out, locals, &mut caps)?;
            }
            _ => return None,
        }
    }
    if outs.is_empty() || caps.len() > MAX_CAPS {
        return None;
    }
    Some(Plan { caps })
}

/// Whether `e` is pure arithmetic over the loop's locals, captures and
/// list reads, free of anything that panics inside the runtime.
fn value_ok(
    e: &Expr,
    temps: &[Local],
    outs: &[Local],
    fresh: &[Local],
    locals: &[Type],
    caps: &mut Vec<Local>,
) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) => true,
        ExprKind::Local(l) => {
            if temps.contains(l) {
                return true;
            }
            !locals[*l as usize].is_ref() && capture(*l, locals, caps).is_some()
        }
        ExprKind::Index(base, i) => {
            let ExprKind::Local(list) = base.kind else {
                return false;
            };
            // Reading a list other chunks write would race.
            if outs.contains(&list) || e.ty.is_ref() {
                return false;
            }
            let _ = fresh;
            capture(list, locals, caps).is_some() && value_ok(i, temps, outs, fresh, locals, caps)
        }
        ExprKind::Binary(_, a, b) | ExprKind::Fast(_, a, b) => {
            value_ok(a, temps, outs, fresh, locals, caps)
                && value_ok(b, temps, outs, fresh, locals, caps)
        }
        ExprKind::Unary(_, x) => value_ok(x, temps, outs, fresh, locals, caps),
        // Only conversions proven to fit: a checked one would panic
        // inside the runtime, where quiet can't reach.
        ExprKind::Convert(x, false) => value_ok(x, temps, outs, fresh, locals, caps),
        ExprKind::Rt(RtOp::MathSqrt, args) => value_ok(&args[0], temps, outs, fresh, locals, caps),
        ExprKind::Rt(RtOp::ListLen, args) => {
            matches!(args[0].kind, ExprKind::Local(l) if capture(l, locals, caps).is_some())
        }
        _ => false,
    }
}

/// Adds `l` to the captures (once). Captures travel as plain `i64`
/// arguments, so only integer and reference slots qualify.
fn capture(l: Local, locals: &[Type], caps: &mut Vec<Local>) -> Option<()> {
    if !matches!(locals[l as usize], Type::I64 | Type::Ref(_)) {
        return None;
    }
    if !caps.contains(&l) {
        caps.push(l);
    }
    Some(())
}

/// An integer reduction: pure lets and exactly one `acc += value`,
/// nothing else. The chunks sum from zero; the caller combines.
pub(super) struct RedPlan {
    pub(super) acc: Local,
    pub(super) caps: Vec<Local>,
    /// `acc = (acc + value) % m` instead of `acc += value`: the flat
    /// sum taken `% m` at the end is the same number, because the
    /// `RemOnce` this shape comes from proves every operand fits and
    /// the value is never negative (see `reduction`).
    pub(super) modulus: Option<i128>,
}

/// Whether this body is an eligible reduction over `loop_vars`.
pub(super) fn reduction(body: &Block, loop_vars: &[Local], locals: &[Type]) -> Option<RedPlan> {
    if body.value.is_some() {
        return None;
    }
    let mut temps = loop_vars.to_vec();
    let mut caps = Vec::new();
    let mut acc = None;
    let mut found_modulus = None;
    for s in &body.stmts {
        match s {
            Stmt::Let(l, e) => {
                if e.ty.is_ref() || !value_ok(e, &temps, &[], &[], locals, &mut caps) {
                    return None;
                }
                temps.push(*l);
            }
            Stmt::Expr(e) => {
                let ExprKind::Assign(a, v) = &e.kind else {
                    return None;
                };
                // `acc += value`, or the modular form `ranges.rs` turned
                // into one proven compare-and-subtract: that proof also
                // means the flat sum `% m` gives the same number.
                let (sum, modulus) = match &v.kind {
                    ExprKind::Fast(FastOp::RemOnce, inner, c) => {
                        let ExprKind::Int(m) = c.kind else {
                            return None;
                        };
                        (inner, Some(m))
                    }
                    _ => (v, None),
                };
                let (ExprKind::Binary(BinOp::Add, left, value)
                | ExprKind::Fast(FastOp::Add, left, value)) = &sum.kind
                else {
                    return None;
                };
                if acc.is_some()
                    || locals[*a as usize] != Type::I64
                    || modulus.is_some_and(|m| m <= 0 || !never_negative(value))
                    || !matches!(&left.kind, ExprKind::Local(x) if x == a)
                    || !value_ok(value, &temps, &[], &[], locals, &mut caps)
                {
                    return None;
                }
                acc = Some(*a);
                found_modulus = modulus;
            }
            _ => return None,
        }
    }
    let acc = acc?;
    // The accumulator stays out of the chunks, and three capture seats
    // belong to the slot lists.
    if caps.contains(&acc) || temps.contains(&acc) || caps.len() > MAX_CAPS - 3 {
        return None;
    }
    Some(RedPlan {
        acc,
        caps,
        modulus: found_modulus,
    })
}

/// Whether `e` is plainly never negative: a non-negative remainder, a
/// proven one-subtract remainder, or a literal.
fn never_negative(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(n) => *n >= 0,
        ExprKind::Fast(FastOp::RemPositive | FastOp::RemSmall | FastOp::RemOnce, _, c) => {
            matches!(c.kind, ExprKind::Int(m) if m > 0)
        }
        _ => false,
    }
}
