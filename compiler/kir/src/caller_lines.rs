// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A panic in the standard library's own code points at the program's
//! call into it (spec 5.6): the line a person can change. Each call from
//! the program's code of a standard library function that can panic
//! sets its thread's call line (`RtOp::CallLineSwap`) and sets it back
//! after (`RtOp::CallLineSet`); a panic at a line of the library shows
//! that line instead (`runtime/native/src/rt.rs`). Calls of functions
//! that can't panic, as most of `std.vec`'s, cost nothing more.
//!
//! The interpreter does the same as a panic leaves such a call
//! (`interp/src/panic_places.rs`), so both give the same line.

use crate::ir::*;
use crate::rt_op::RtOp;
use crate::walk::{Part, any_expr, any_in_block, parts, stmt_parts};

/// Marks the calls in `functions` of library functions that can panic.
/// A function from line `std_from` of the program on is the library's.
pub fn mark_calls(functions: &mut [Function], std_from: u32) {
    let std: Vec<bool> = functions.iter().map(|f| f.line >= std_from).collect();
    if !std.contains(&true) {
        return;
    }
    let panics = can_panic(functions);
    let gpu = on_the_gpu(functions);
    for (i, f) in functions.iter_mut().enumerate() {
        if std[i] || gpu[i] || f.external.is_some() {
            continue;
        }
        let wraps = |id: FuncId| std[id as usize] && panics[id as usize];
        let mut pass = Pass {
            locals: &mut f.locals,
            wraps: &wraps,
        };
        pass.block(&mut f.body);
    }
}

/// Whether each function can panic: it has an operation that can, or
/// calls one that can. A function value or a runtime operation is taken
/// to, as is an integer operation that can overflow.
fn can_panic(functions: &[Function]) -> Vec<bool> {
    let own = |e: &Expr| match &e.kind {
        ExprKind::Binary(op, a, _) => a.ty.is_int() && !never_overflows(*op),
        ExprKind::Unary(UnOp::Neg, x) => x.ty.is_int(),
        ExprKind::Convert(_, checked) => *checked,
        ExprKind::Index(..)
        | ExprKind::Unwrap(_)
        | ExprKind::Panic(_)
        | ExprKind::Rt(..)
        | ExprKind::CallValue(..)
        | ExprKind::Parse(_) => true,
        _ => false,
    };
    let mut panics: Vec<bool> = (functions.iter())
        .map(|f| any_in_block(&f.body, &mut |e| own(e)) || has_check(&f.body))
        .collect();
    loop {
        let mut changed = false;
        for (i, f) in functions.iter().enumerate() {
            if panics[i] {
                continue;
            }
            let calls = |e: &Expr| matches!(e.kind, ExprKind::Call(id, _) if panics[id as usize]);
            if any_in_block(&f.body, &mut { calls }) {
                panics[i] = true;
                changed = true;
            }
        }
        if !changed {
            return panics;
        }
    }
}

fn never_overflows(op: BinOp) -> bool {
    use BinOp::*;
    matches!(op, Eq | Ne | Lt | Le | Gt | Ge | BitAnd | BitOr | BitXor)
}

/// Whether a block has a `Check` (a contract or an `assert`) at any depth.
/// `any_in_block` reaches every block inside, so each block's own
/// statements are looked at once: looking at a branch's whole depth
/// again for each `if` would take time doubling with each arm of a
/// `match`.
fn has_check(block: &Block) -> bool {
    fn in_stmts(block: &Block) -> bool {
        block.stmts.iter().any(|s| match s {
            Stmt::Check { .. } => true,
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
                in_stmts(body)
            }
            _ => false,
        })
    }
    in_stmts(block)
        || any_in_block(block, &mut |e| match &e.kind {
            ExprKind::If(_, a, b) | ExprKind::Fallback(a, b) => in_stmts(a) || in_stmts(b),
            ExprKind::Block(b) => in_stmts(b),
            _ => false,
        })
}

/// The functions a GPU kernel may be made of (spec 15): the functions
/// given to a GPU operation, and what they call. They stay as written,
/// since a kernel has no runtime to call.
fn on_the_gpu(functions: &[Function]) -> Vec<bool> {
    let mut gpu = vec![false; functions.len()];
    let mut todo: Vec<FuncId> = Vec::new();
    for f in functions {
        any_in_block(&f.body, &mut |e| {
            let gpu_op = |op: &RtOp| {
                let name = format!("{op:?}");
                name.starts_with("Gpu") || name.starts_with("Draw")
            };
            if let ExprKind::Rt(op, args) = &e.kind
                && gpu_op(op)
            {
                for a in args {
                    any_expr(a, &mut |x| {
                        if let ExprKind::Closure { func, .. } = x.kind {
                            todo.push(func);
                        }
                        false
                    });
                }
            }
            false
        });
    }
    while let Some(id) = todo.pop() {
        if std::mem::replace(&mut gpu[id as usize], true) {
            continue;
        }
        any_in_block(&functions[id as usize].body, &mut |e| {
            if let ExprKind::Call(callee, _) | ExprKind::Closure { func: callee, .. } = e.kind {
                todo.push(callee);
            }
            false
        });
    }
    gpu
}

struct Pass<'p, W: Fn(FuncId) -> bool> {
    locals: &'p mut Vec<Type>,
    wraps: &'p W,
}

impl<W: Fn(FuncId) -> bool> Pass<'_, W> {
    fn local(&mut self, ty: Type) -> Local {
        self.locals.push(ty);
        (self.locals.len() - 1) as Local
    }

    /// `let old = swap(line)`, and the statement that sets it back.
    fn around(&mut self, line: u32) -> (Stmt, Stmt) {
        let old = self.local(Type::I64);
        let at = |kind, ty| Expr { kind, ty, line };
        let swap = at(
            ExprKind::Rt(
                RtOp::CallLineSwap,
                vec![at(ExprKind::Int(line.into()), Type::I64)],
            ),
            Type::I64,
        );
        let back = at(
            ExprKind::Rt(RtOp::CallLineSet, vec![at(ExprKind::Local(old), Type::I64)]),
            Type::Unit,
        );
        (Stmt::Let(old, swap), Stmt::Expr(back))
    }

    fn block(&mut self, b: &mut Block) {
        for mut s in std::mem::take(&mut b.stmts) {
            // A call that can fail stays the call of its `LetResult`.
            if let Stmt::LetResult { call, .. } = &mut s
                && let ExprKind::Call(id, args) = &mut call.kind
                && (self.wraps)(*id)
            {
                args.iter_mut().for_each(|a| self.expr(a));
                let (set, back) = self.around(call.line);
                b.stmts.extend([set, s, back]);
                continue;
            }
            stmt_parts(&mut s, &mut |p| match p {
                Part::Expr(e) => self.expr(e),
                Part::Block(inner) => self.block(inner),
            });
            b.stmts.push(s);
        }
        if let Some(v) = &mut b.value {
            self.expr(v);
        }
    }

    fn expr(&mut self, e: &mut Expr) {
        parts(e, &mut |p| match p {
            Part::Expr(x) => self.expr(x),
            Part::Block(inner) => self.block(inner),
        });
        let ExprKind::Call(id, _) = e.kind else {
            return;
        };
        if !(self.wraps)(id) {
            return;
        }
        let (ty, line) = (e.ty, e.line);
        let call = std::mem::replace(
            e,
            Expr {
                kind: ExprKind::None,
                ty,
                line,
            },
        );
        let (set, back) = self.around(line);
        let mut stmts = vec![set];
        let value = if ty == Type::Unit {
            stmts.push(Stmt::Expr(call));
            None
        } else {
            let result = self.local(ty);
            stmts.push(Stmt::Let(result, call));
            Some(Box::new(Expr {
                kind: ExprKind::Local(result),
                ty,
                line,
            }))
        };
        stmts.push(back);
        *e = Expr {
            kind: ExprKind::Block(Block { stmts, value }),
            ty,
            line,
        };
    }
}

#[cfg(test)]
mod tests;
