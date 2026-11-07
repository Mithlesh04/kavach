//! A closure's function borrows its closure (`lower_closure.rs`): the
//! caller keeps its reference while the call runs, as a C++ program
//! calls a `std::function`, so a call through a function value changes
//! no reference count. The captures the function reads from its closure
//! at its start borrow too, when nothing in it gives them another value:
//! the closure holds them for the whole call. Both backends and the
//! runtime's callers (a task's start, a `drop`) keep to this.

use crate::ir::*;
use crate::walk::any_in_block;

/// Which functions are closures' functions: those a `Closure` holds (not
/// a C callback's, which has no closure: spec 10.7).
pub fn closure_functions(functions: &[Function]) -> Vec<bool> {
    let mut found = vec![false; functions.len()];
    for f in functions {
        any_in_block(&f.body, &mut |e| {
            if let ExprKind::Closure { func, env, .. } = e.kind
                && env != Type::Unit
                && let Some(slot) = found.get_mut(func as usize)
            {
                *slot = true;
            }
            false
        });
    }
    found
}

/// The locals of closure function `f` that borrow: its closure (its first
/// parameter), and each capture read from it at the start of its body
/// that nothing else gives a value. A move out of one only reads it.
pub fn borrowing_locals(f: &Function) -> Vec<Local> {
    let Some(&closure) = f.params.first() else {
        return Vec::new();
    };
    let mut out = vec![closure];
    for (i, s) in f.body.stmts.iter().enumerate() {
        let Stmt::Let(l, value) = s else {
            break;
        };
        let ExprKind::GetField(base, _) = &value.kind else {
            break;
        };
        if !matches!(base.kind, ExprKind::Local(c) | ExprKind::Move(c) if c == closure) {
            break;
        }
        let rest = Block {
            stmts: f.body.stmts[i + 1..].to_vec(),
            value: f.body.value.clone(),
        };
        if f.locals[*l as usize].is_ref() && !given(&rest, *l) && !f.params.contains(l) {
            out.push(*l);
        }
    }
    out
}

/// Whether `block` gives local `l` a value: a `let`, an assignment or a
/// loop variable (not a move out of it).
fn given(block: &Block, l: Local) -> bool {
    fn in_stmts(block: &Block, l: Local) -> bool {
        block.stmts.iter().any(|s| match s {
            Stmt::Let(x, _) => *x == l,
            Stmt::LetResult { value, error, .. } => *value == l || *error == l,
            Stmt::ForRange { var, body, .. } => *var == l || in_stmts(body, l),
            Stmt::ForEach {
                var, index, body, ..
            } => *var == l || *index == l || in_stmts(body, l),
            Stmt::While { body, .. } => in_stmts(body, l),
            _ => false,
        })
    }
    in_stmts(block, l)
        || any_in_block(block, &mut |e| match &e.kind {
            ExprKind::Assign(x, _) => *x == l,
            ExprKind::If(_, then, otherwise) | ExprKind::Fallback(then, otherwise) => {
                in_stmts(then, l) || in_stmts(otherwise, l)
            }
            ExprKind::Block(b) => in_stmts(b, l),
            _ => false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(kind: ExprKind, ty: Type) -> Expr {
        Expr { kind, ty, line: 1 }
    }

    /// `x => inner(x) + 1`: its closure and `inner` borrow; a capture the
    /// body assigns doesn't.
    #[test]
    fn a_closure_and_its_unchanged_captures_borrow() {
        let (env, func) = (Type::Ref(0), Type::Ref(1));
        let read = |i| {
            e(
                ExprKind::GetField(Box::new(e(ExprKind::Local(0), env)), i),
                func,
            )
        };
        let f = Function {
            name: "lambda".into(),
            params: vec![0, 1],
            ret: Type::I64,
            locals: vec![env, Type::I64, func, func],
            body: Block {
                stmts: vec![
                    Stmt::Let(2, read(1)),
                    Stmt::Let(3, read(2)),
                    Stmt::Expr(e(
                        ExprKind::Assign(3, Box::new(e(ExprKind::Local(2), func))),
                        Type::Unit,
                    )),
                ],
                value: None,
            },
            line: 1,
            fails: false,
            external: None,
        };
        assert_eq!(borrowing_locals(&f), vec![0, 2]);
    }
}
