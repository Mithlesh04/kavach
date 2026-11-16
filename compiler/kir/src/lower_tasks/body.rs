//! The function a spawned task runs (spec 9, `lower_tasks.rs`): its
//! captured values, the parts of its call read before it started
//! (`args.rs`), then `x`, given back or done.

use super::at;
use crate::ir::{Block, ExprKind as K, Function, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast as ast;
use kavach_checker::Ty;

/// The parts read before the task, by span, with their types.
pub(super) type Held = [((u32, u32), Type)];

impl<'a> Lowerer<'a> {
    /// The function `x` runs in: `(closure) -> value ! Error`.
    pub(super) fn task_body(
        &mut self,
        x: &'a ast::Expr,
        env_ty: Type,
        value_ty: Type,
        captured: &[kavach_resolver::DefId],
        (types, held): (&[Type], &Held),
        line: u32,
    ) -> Lowered<Function> {
        let (env, mut body) = self.unpack_env(env_ty, captured, types, line);
        // The parts read before the task, after the captured variables.
        for (i, (span, t)) in held.iter().enumerate() {
            let local = self.new_local(*t);
            let slot = (types.len() + i) as u32 + 1;
            let read = K::GetField(Box::new(self.local_expr(env, line)), slot);
            body.stmts.push(Stmt::Let(local, at(read, *t, line)));
            self.held_parts.insert(*span, local);
        }
        let lowered = self.task_body_value(x, value_ty, body, line);
        for (span, _) in held {
            self.held_parts.remove(span);
        }
        let body = lowered?;
        Ok(Function {
            name: "task".into(),
            params: vec![env],
            ret: value_ty,
            locals: self.locals.clone(),
            body,
            line,
            fails: true,
            external: None,
            variadic: false,
        })
    }

    /// The task's body after its captured values: `x`, given back or done.
    fn task_body_value(
        &mut self,
        x: &'a ast::Expr,
        value_ty: Type,
        mut body: Block,
        line: u32,
    ) -> Lowered<Block> {
        let fails = matches!(
            self.c.expr_types.get(&(x.span.start, x.span.end)),
            Some(Ty::Result(..))
        );
        match &x.kind {
            // Code run for what it does: its last line is a statement.
            ast::ExprKind::Block(b) if value_ty == Type::Unit && !fails => {
                self.block_into(b, false, &mut body)?;
            }
            _ if value_ty == Type::Unit && !fails => self.expr_stmt(x, &mut body.stmts)?,
            _ => {
                let value = if fails {
                    self.try_expr(x, None, line)?
                } else {
                    self.expr(x)?
                };
                if value_ty == Type::Unit {
                    body.stmts.push(Stmt::Expr(value));
                } else {
                    body.value = Some(Box::new(self.coerce(value, value_ty)?));
                }
            }
        }
        Ok(body)
    }
}
