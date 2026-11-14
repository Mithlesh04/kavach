//! Blocks given to a call (spec 4.8, decision 0107): `log.span(name,
//! from) { … }` and `random.seeded(seed) { … }`. The call's op puts an
//! entry on the task's scope stack (`runtime/native/src/rt_scopes.rs`)
//! and gives its depth; `FsScopeLeave` goes back to it however the block
//! ends:
//! - at its end, here;
//! - by `break` or `continue` to a loop outside it, in `jump`
//!   (`lower_labels.rs`), through `leave_for_jump`;
//! - by `return`, `try` or `fail`, which leave the function: each function
//!   holding such a block goes back to the depth it started at however it
//!   ends (`scope_blocks` below, at the passes).

use crate::ir::{self, ExprKind as K, Function, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn scoped(
        &mut self,
        call: &'a ast::Expr,
        body: &'a ast::Block,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let ast::ExprKind::Call { callee, args, .. } = &call.kind else {
            return Err(self.unsupported("this block", call.span));
        };
        let seeded =
            matches!(&callee.kind, ast::ExprKind::Field { name, .. } if name.name == "seeded");
        let (op, wanted): (RtOp, &[&str]) = if seeded {
            (RtOp::ScopeSeeded, &["seed"])
        } else {
            (RtOp::ScopeSpan, &["name", "from"])
        };
        let mut given: Vec<Option<&'a ast::Expr>> = vec![None; wanted.len()];
        for (i, a) in args.iter().enumerate() {
            let at = match &a.name {
                Some(n) => wanted.iter().position(|w| *w == n.name),
                None => Some(i),
            };
            if let Some(slot) = at.and_then(|p| given.get_mut(p)) {
                *slot = Some(&a.value);
            }
        }
        let mut operands = Vec::new();
        for g in given {
            operands.push(match g {
                Some(e) => self.expr(e)?,
                // No `from`: 0, no text.
                None => self.at(K::Int(0), Type::I64, line),
            });
        }
        let depth = self.new_local(Type::I64);
        let mut block = ir::Block::default();
        block.stmts.push(Stmt::Let(
            depth,
            self.at(K::Rt(op, operands), Type::I64, line),
        ));
        self.scoped_depths.push((depth, self.loop_frames.len()));
        let mut inner = ir::Block::default();
        let lowered = self.block_into(body, ty != Type::Unit, &mut inner);
        self.scoped_depths.pop();
        lowered?;
        // The value stays inside the block: its locals are let go at its
        // end, and the value may read them.
        let result = self.new_local(ty);
        let has_value = inner.value.is_some();
        let inner = self.at(
            K::Block(inner),
            if has_value { ty } else { Type::Unit },
            line,
        );
        block.stmts.push(match has_value {
            true => Stmt::Let(result, inner),
            false => Stmt::Expr(inner),
        });
        block.stmts.push(Stmt::Expr(self.leave(depth, line)));
        if ty != Type::Unit {
            block.value = Some(Box::new(self.local_expr(result, line)));
        }
        Ok(self.at(K::Block(block), ty, line))
    }

    /// `FsScopeLeave` back to the depth in local `depth`.
    fn leave(&self, depth: ir::Local, line: u32) -> ir::Expr {
        let back = vec![
            self.local_expr(depth, line),
            self.at(K::Int(0), Type::I64, line),
        ];
        self.at(K::Rt(RtOp::FsScopeLeave, back), Type::Unit, line)
    }

    /// Before a jump to loop number `target`: leaving the outermost block
    /// given to a call that the jump leaves, if any.
    pub(crate) fn leave_for_jump(&self, target: usize, out: &mut Vec<Stmt>) {
        if let Some((depth, _)) = self.scoped_depths.iter().find(|(_, loops)| target < *loops) {
            out.push(Stmt::Expr(self.leave(*depth, 0)));
        }
    }
}

/// The functions holding a block given to a call: each goes back to the
/// scope depth it started at however it ends.
pub(crate) fn scope_blocks(functions: &[Function]) -> Vec<usize> {
    let starts = |e: &ir::Expr| matches!(e.kind, K::Rt(RtOp::ScopeSpan | RtOp::ScopeSeeded, _));
    (functions.iter().enumerate())
        .filter(|(_, f)| crate::walk::any_in_block(&f.body, &mut |e| starts(e)))
        .map(|(i, _)| i)
        .collect()
}
