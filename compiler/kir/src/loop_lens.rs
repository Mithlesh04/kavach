//! `while i < data.len()`: the length of a variable the loop never sets
//! is the same every round, as values never change behind a name
//! (spec 6.1). So it is read once, before the loop, into a local of its
//! own (Sakshi's S8: `Bytes.len()` in the condition was a runtime call each
//! round, 1477 ms over 72 MB where reading it first took 71 ms).

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{blocks_mut, sets_local};

/// Lengths worth reading once: a call into the runtime each time, or a
/// walk over the text for `String`'s count of characters.
fn length_op(op: RtOp) -> bool {
    matches!(op, RtOp::BytesLen | RtOp::StrLen | RtOp::ListLen)
}

pub fn hoist_loop_lens(f: &mut Function) {
    let mut locals = std::mem::take(&mut f.locals);
    blocks_mut(&mut f.body, &mut |b| {
        let mut i = 0;
        while i < b.stmts.len() {
            let before = match &mut b.stmts[i] {
                Stmt::While { cond, body } => hoist(cond, body, &mut locals),
                _ => Vec::new(),
            };
            let n = before.len();
            b.stmts.splice(i..i, before);
            i += n + 1;
        }
    });
    f.locals = locals;
}

/// Each length in `cond` of a local `body` never sets: replaced by a new
/// local, set by the statements given back.
fn hoist(cond: &mut Expr, body: &Block, locals: &mut Vec<Type>) -> Vec<Stmt> {
    let mut out = Vec::new();
    let mut seen: Vec<(RtOp, Local, Local)> = Vec::new();
    replace(cond, &mut |e| {
        let ExprKind::Rt(op, args) = &e.kind else {
            return None;
        };
        let [arg] = &args[..] else { return None };
        let ExprKind::Local(l) = arg.kind else {
            return None;
        };
        if !length_op(*op) || sets_local(body, l) {
            return None;
        }
        if let Some(&(_, _, held)) = seen.iter().find(|(o, x, _)| o == op && *x == l) {
            return Some(held);
        }
        let held = locals.len() as Local;
        locals.push(e.ty);
        out.push(Stmt::Let(held, e.clone()));
        seen.push((*op, l, held));
        Some(held)
    });
    out
}

/// Replaces each part of `e` that `f` gives a local for, by that local.
fn replace(e: &mut Expr, f: &mut impl FnMut(&Expr) -> Option<Local>) {
    if let Some(l) = f(e) {
        e.kind = ExprKind::Local(l);
        return;
    }
    match &mut e.kind {
        ExprKind::Binary(_, a, b) | ExprKind::And(a, b) | ExprKind::Or(a, b) => {
            replace(a, f);
            replace(b, f);
        }
        ExprKind::Unary(_, a) => replace(a, f),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{ExprKind, Stmt};
    use crate::rt_op::RtOp;

    /// The ops each `while` condition of `src`'s functions still calls.
    fn lens_in_conditions(src: &str) -> Vec<RtOp> {
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        let mut found = Vec::new();
        for f in &program.functions {
            crate::walk::blocks_mut(&mut f.body.clone(), &mut |b| {
                for s in &b.stmts {
                    if let Stmt::While { cond, .. } = s {
                        crate::walk::any_expr(cond, &mut |e| {
                            if let ExprKind::Rt(op, _) = &e.kind {
                                found.push(*op);
                            }
                            false
                        });
                    }
                }
            });
        }
        found
    }

    #[test]
    fn a_length_the_loop_never_changes_is_read_once() {
        let src = "fn count(data: Bytes) -> Int {\n    let i = 0\n    while i < data.len() {\n        i += 1\n    }\n    i\n}\n\nfn main() {\n    print(count(Bytes([1, 2])))\n}\n";
        assert!(!lens_in_conditions(src).contains(&RtOp::BytesLen));
    }

    #[test]
    fn a_length_the_loop_changes_is_read_each_round() {
        let src = "fn main() {\n    let s = \"ab\"\n    let k = 0\n    while k < s.len() {\n        k += 1\n        if k == 1 {\n            s = s + \"!\"\n        }\n    }\n    print(k)\n}\n";
        assert!(lens_in_conditions(src).contains(&RtOp::StrLen));
    }
}
