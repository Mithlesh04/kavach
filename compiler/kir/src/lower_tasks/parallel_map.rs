//! `items.parallel_map(f, max: m)` (spec 9.3): a loop that starts a
//! task for each item, around a closure holding the function and the
//! item, then a loop that waits for each task in order.

use super::{at, local};
use crate::ir::{self, Block, ExprKind as K, Function, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `items.parallel_map(f, max: m)` (spec 9.3), as the interpreter
    /// runs it: each item in a task of its own; before starting task
    /// `i`, task `i - m` is waited for (0: no limit); then every result
    /// in order.
    pub(crate) fn parallel_map(
        &mut self,
        list: ir::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let (Some(Shape::List(item_ty)), Some(Shape::List(out_item))) = (
            self.shapes.shape(list.ty).cloned(),
            self.shapes.shape(ty).cloned(),
        ) else {
            return Err(self.unsupported("`parallel_map` here", span));
        };
        let Some(f) = args.iter().find(|a| a.name.is_none()) else {
            return Err(self.unsupported("`parallel_map` without a function", span));
        };
        let f = &f.value;
        let max = match args
            .iter()
            .find(|a| a.name.as_ref().is_some_and(|n| n.name == "max"))
        {
            Some(a) => self.expr(&a.value)?,
            None => at(K::Int(0), Type::I64, line),
        };
        let fn_ty = match self.c.expr_types.get(&(f.span.start, f.span.end)).cloned() {
            Some(t) => self.kir_type(&t, f.span)?,
            None => return Err(self.unsupported("this function", f.span)),
        };
        let fv = match &f.kind {
            ast::ExprKind::Lambda { .. } | ast::ExprKind::DotLambda(_) => {
                self.closure(f, fn_ty, line)?
            }
            _ => self.expr(f)?,
        };
        // Each task's closure holds the function and its item.
        let env_ty = self.shapes_intern(Shape::Closure(vec![fn_ty, item_ty]));
        let saved = self.save_function(out_item);
        let env = self.new_local(env_ty);
        let field =
            |i: u32, t: Type| at(K::GetField(Box::new(local(env, env_ty, line)), i), t, line);
        let call = at(
            K::CallValue(Box::new(field(1, fn_ty)), vec![field(2, item_ty)]),
            out_item,
            line,
        );
        let body = Function {
            name: "parallel_map task".into(),
            params: vec![env],
            ret: out_item,
            locals: self.locals.clone(),
            body: Block {
                stmts: Vec::new(),
                value: Some(Box::new(call)),
            },
            line,
            fails: true,
            external: None,
        };
        self.restore_function(saved);
        let body = self.add_function(body);
        let saved = self.save_function(Type::Unit);
        let error_ty = self.error_type();
        let thunk = self.task_thunk(body, env_ty, out_item, error_ty, line);
        self.restore_function(saved);
        let thunk = self.add_function(thunk);

        let task_ty = self.shapes_intern(Shape::Task(out_item));
        let tasks_ty = self.list_type(task_ty);
        let error_ty = self.error_type();
        let (f_local, max_local, src) = (
            self.new_local(fn_ty),
            self.new_local(Type::I64),
            self.new_local(list.ty),
        );
        let (tasks, item, index) = (
            self.new_local(tasks_ty),
            self.new_local(item_ty),
            self.new_local(Type::I64),
        );
        let (out, task, j) = (
            self.new_local(ty),
            self.new_local(task_ty),
            self.new_local(Type::I64),
        );
        let (value, error, skipped) = (
            self.new_local(out_item),
            self.new_local(error_ty),
            self.new_local(out_item),
        );
        let int = |n| at(K::Int(n), Type::I64, line);
        let wait = |t: ir::Expr| at(K::Rt(RtOp::TaskWait, vec![t]), out_item, line);
        let push = |acc: Local, acc_ty: Type, v: ir::Expr| {
            let pushed = at(
                K::Rt(RtOp::ListPush, vec![local(acc, acc_ty, line), v]),
                acc_ty,
                line,
            );
            Stmt::Expr(at(K::Assign(acc, Box::new(pushed)), Type::Unit, line))
        };
        // Before task `i`: wait for task `i - max`.
        let full = at(
            K::And(
                Box::new(at(
                    K::Binary(
                        ir::BinOp::Gt,
                        Box::new(local(max_local, Type::I64, line)),
                        Box::new(int(0)),
                    ),
                    Type::Bool,
                    line,
                )),
                Box::new(at(
                    K::Binary(
                        ir::BinOp::Ge,
                        Box::new(local(index, Type::I64, line)),
                        Box::new(local(max_local, Type::I64, line)),
                    ),
                    Type::Bool,
                    line,
                )),
            ),
            Type::Bool,
            line,
        );
        let earlier = at(
            K::Index(
                Box::new(local(tasks, tasks_ty, line)),
                Box::new(at(
                    K::Binary(
                        ir::BinOp::Sub,
                        Box::new(local(index, Type::I64, line)),
                        Box::new(local(max_local, Type::I64, line)),
                    ),
                    Type::I64,
                    line,
                )),
            ),
            task_ty,
            line,
        );
        let wait_earlier = K::If(
            Box::new(full),
            Block {
                stmts: vec![Stmt::LetResult {
                    value: skipped,
                    error,
                    call: wait(earlier),
                }],
                value: None,
            },
            Block::default(),
        );
        let closure = K::Closure {
            func: thunk,
            env: env_ty,
            captures: vec![local(f_local, fn_ty, line), local(item, item_ty, line)],
        };
        let func_ty = self.shapes_intern(Shape::Func(Vec::new(), Type::Unit));
        let spawned = at(
            K::Rt(RtOp::TaskSpawn, vec![at(closure, func_ty, line)]),
            task_ty,
            line,
        );
        let start = Stmt::ForEach {
            var: item,
            index,
            list: src,
            body: Block {
                stmts: vec![
                    Stmt::Expr(at(wait_earlier, Type::Unit, line)),
                    push(tasks, tasks_ty, spawned),
                ],
                value: None,
            },
        };
        let collect = Stmt::ForEach {
            var: task,
            index: j,
            list: tasks,
            body: Block {
                stmts: vec![
                    Stmt::LetResult {
                        value,
                        error,
                        call: wait(local(task, task_ty, line)),
                    },
                    push(out, ty, local(value, out_item, line)),
                ],
                value: None,
            },
        };
        let stmts = vec![
            Stmt::Let(f_local, fv),
            Stmt::Let(max_local, max),
            Stmt::Let(src, list),
            Stmt::Let(tasks, at(K::List(Vec::new()), tasks_ty, line)),
            start,
            Stmt::Let(out, at(K::List(Vec::new()), ty, line)),
            collect,
        ];
        Ok(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(local(out, ty, line))),
            }),
            ty,
            line,
        ))
    }
}
