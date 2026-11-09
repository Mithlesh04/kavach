//! Tasks (spec 9, decision 0020). `spawn x` becomes two functions and a
//! closure the runtime starts on a task's thread:
//!
//! - the *body*: `x`, in a function of its own that can fail (a failing
//!   call in `x` passes its error on, as with `try`), reading the
//!   variables `x` uses from the closure, like a lambda;
//! - the *thunk*: calls the body and hands its value or error to the
//!   runtime (`TaskDone`, `TaskFail`). It returns nothing, so the runtime
//!   calls it as a plain `fn(closure)`, the same on every platform.
//!
//! A function that spawns waits for its tasks when it returns, however it
//! returns (spec 9.2): `join_tasks` wraps it in one that marks, calls it,
//! and waits for the tasks spawned after the mark.

use crate::ir::{self, Block, ExprKind as K, FuncId, Function, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use crate::walk::any_in_block;
use kavach_ast as ast;
use kavach_checker::Ty;

mod channels;
mod parallel_map;

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

fn local(l: Local, ty: Type, line: u32) -> ir::Expr {
    at(K::Local(l), ty, line)
}

impl<'a> Lowerer<'a> {
    /// `spawn x`, of type `ty` (a `Task`), run as `kind` says (spec 9.8).
    /// `spawn x`, or `spawn x within d` (spec 9.6): the spawned task, then
    /// its time limit.
    pub(crate) fn spawn_within(
        &mut self,
        kind: ast::SpawnKind,
        x: &'a ast::Expr,
        within: Option<&'a ast::Expr>,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let task = self.spawn(kind, x, ty, line)?;
        let Some(d) = within else {
            return Ok(task);
        };
        let d = self.expr(d)?;
        let ms = at(K::GetField(Box::new(d), 0), Type::I64, line);
        Ok(at(K::Rt(RtOp::TaskWithin, vec![task, ms]), ty, line))
    }

    pub(crate) fn spawn(
        &mut self,
        kind: ast::SpawnKind,
        x: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Shape::Task(value_ty)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("this `spawn`", x.span));
        };
        let (captured, outer, types) = self.captures(x);
        let values: Vec<ir::Expr> = outer.iter().map(|l| self.local_expr(*l, line)).collect();
        let env_ty = self.shapes_intern(Shape::Closure(types.clone()));

        let saved = self.save_function(value_ty);
        let body = self.task_body(x, env_ty, value_ty, &captured, &types, line);
        self.restore_function(saved);
        let body = self.add_function(body?);

        let saved = self.save_function(Type::Unit);
        let error_ty = self.error_type();
        let thunk = self.task_thunk(body, env_ty, value_ty, error_ty, line);
        self.restore_function(saved);
        let thunk = self.add_function(thunk);

        let func_ty = self.shapes_intern(Shape::Func(Vec::new(), Type::Unit));
        let closure = K::Closure {
            func: thunk,
            env: env_ty,
            captures: values,
        };
        // The runtime's codes for the kinds (`runtime/native/src/rt_tasks.rs`).
        let how = match kind {
            ast::SpawnKind::Coroutine => 0,
            ast::SpawnKind::Thread => 1,
            ast::SpawnKind::Cores => 2,
            ast::SpawnKind::Process => return self.spawn_process(x, ty, line),
        };
        let args = vec![at(closure, func_ty, line), at(K::Int(how), Type::I64, line)];
        Ok(at(K::Rt(RtOp::TaskSpawn, args), ty, line))
    }

    /// The function `x` runs in: `(closure) -> value ! Error`.
    fn task_body(
        &mut self,
        x: &'a ast::Expr,
        env_ty: Type,
        value_ty: Type,
        captured: &[kavach_resolver::DefId],
        types: &[Type],
        line: u32,
    ) -> Lowered<Function> {
        let (env, mut body) = self.unpack_env(env_ty, captured, types, line);
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
        Ok(Function {
            name: "task".into(),
            params: vec![env],
            ret: value_ty,
            locals: self.locals.clone(),
            body,
            line,
            fails: true,
            external: None,
        })
    }

    /// `time.after(d, f)` (spec 14.4): a task, of the caller's, that sleeps
    /// for `d` and then calls `f`.
    pub(crate) fn time_after(
        &mut self,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let (Some(d), Some(f), Some(Shape::Task(value_ty))) =
            (args.first(), args.get(1), self.shapes.shape(ty).cloned())
        else {
            return Err(self.unsupported("`time.after` here", span));
        };
        let d = self.expr(&d.value)?;
        let ms = at(K::GetField(Box::new(d), 0), Type::I64, line);
        let f = &f.value;
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
        let env_ty = self.shapes_intern(Shape::Closure(vec![fn_ty, Type::I64]));
        let saved = self.save_function(value_ty);
        let env = self.new_local(env_ty);
        let field =
            |i: u32, t: Type| at(K::GetField(Box::new(local(env, env_ty, line)), i), t, line);
        let sleep = at(
            K::Rt(RtOp::TimeSleepMs, vec![field(2, Type::I64)]),
            Type::Unit,
            line,
        );
        let call = at(
            K::CallValue(Box::new(field(1, fn_ty)), Vec::new()),
            value_ty,
            line,
        );
        let mut body = Block {
            stmts: vec![Stmt::Expr(sleep)],
            value: None,
        };
        if value_ty == Type::Unit {
            body.stmts.push(Stmt::Expr(call));
        } else {
            body.value = Some(Box::new(call));
        }
        let body = Function {
            name: "time.after task".into(),
            params: vec![env],
            ret: value_ty,
            locals: self.locals.clone(),
            body,
            line,
            fails: true,
            external: None,
        };
        self.restore_function(saved);
        let body = self.add_function(body);
        let saved = self.save_function(Type::Unit);
        let error_ty = self.error_type();
        let thunk = self.task_thunk(body, env_ty, value_ty, error_ty, line);
        self.restore_function(saved);
        let thunk = self.add_function(thunk);
        let func_ty = self.shapes_intern(Shape::Func(Vec::new(), Type::Unit));
        let closure = K::Closure {
            func: thunk,
            env: env_ty,
            captures: vec![fv, ms],
        };
        Ok(at(
            K::Rt(RtOp::TaskSpawn, vec![at(closure, func_ty, line)]),
            ty,
            line,
        ))
    }

    /// The function the runtime calls: runs the task's body and hands
    /// over its value or error.
    pub(crate) fn task_thunk(
        &mut self,
        body: FuncId,
        env_ty: Type,
        value_ty: Type,
        error_ty: Type,
        line: u32,
    ) -> Function {
        let env = self.new_local(env_ty);
        let (value, error) = (self.new_local(value_ty), self.new_local(error_ty));
        let call = at(
            K::Call(body, vec![local(env, env_ty, line)]),
            value_ty,
            line,
        );
        let done = if value_ty == Type::Unit {
            at(K::Int(0), Type::I64, line)
        } else {
            local(value, value_ty, line)
        };
        let failed = at(
            K::IsSome(Box::new(local(error, error_ty, line))),
            Type::Bool,
            line,
        );
        let hand_over = |op, arg| Block {
            stmts: vec![Stmt::Expr(at(K::Rt(op, vec![arg]), Type::Unit, line))],
            value: None,
        };
        let choice = K::If(
            Box::new(failed),
            hand_over(RtOp::TaskFail, local(error, error_ty, line)),
            hand_over(RtOp::TaskDone, done),
        );
        Function {
            name: "task thunk".into(),
            params: vec![env],
            ret: Type::Unit,
            locals: self.locals.clone(),
            body: Block {
                stmts: vec![
                    Stmt::LetResult { value, error, call },
                    Stmt::Expr(at(choice, Type::Unit, line)),
                ],
                value: None,
            },
            line,
            fails: false,
            external: None,
        }
    }
}

/// Wraps each function that spawns tasks so that it waits for them when
/// it returns (spec 9.2): `mark; result = body(params); join(mark)`. A
/// durable agent's `start` (its `JournalOpen`) may leave replayed events
/// going on as tasks of its caller (decision 0072).
pub(crate) fn join_tasks(functions: &mut Vec<Function>, error_ty: Type) {
    for id in 0..functions.len() {
        let spawns = any_in_block(&functions[id].body, &mut |e| {
            matches!(e.kind, K::Rt(RtOp::TaskSpawn | RtOp::JournalOpen, _))
        });
        if spawns {
            around(
                functions,
                id,
                error_ty,
                RtOp::TaskMark,
                None,
                RtOp::TaskJoin,
            );
        }
    }
}

/// Wraps each function with `uses Fs("folder")` (spec 8.1) so that file
/// access is kept inside the folder while it runs, however it returns:
/// `depth = enter(folder); result = body(params); leave(depth)`.
pub(crate) fn scope_files(
    functions: &mut Vec<Function>,
    scoped: &[(usize, String)],
    error_ty: Type,
    text_ty: Type,
) {
    for (id, folder) in scoped {
        let line = functions[*id].line;
        let folder = at(K::Str(folder.clone()), text_ty, line);
        around(
            functions,
            *id,
            error_ty,
            RtOp::FsScopeEnter,
            Some(folder),
            RtOp::FsScopeLeave,
        );
    }
}

/// Turns function `id` into one that runs `enter` (with `arg`, if any),
/// calls a new function holding its body, runs `exit` with what `enter`
/// gave, and returns the body's value or passes its error on.
fn around(
    functions: &mut Vec<Function>,
    id: usize,
    error_ty: Type,
    enter: RtOp,
    arg: Option<ir::Expr>,
    exit: RtOp,
) {
    let inner_id = functions.len() as FuncId;
    let f = &mut functions[id];
    let line = f.line;
    let param_types: Vec<Type> = f.params.iter().map(|p| f.locals[*p as usize]).collect();
    let inner = Function {
        name: format!("{} body", f.name),
        params: f.params.clone(),
        ret: f.ret,
        locals: f.locals.clone(),
        body: std::mem::take(&mut f.body),
        line,
        fails: f.fails,
        external: None,
    };
    let mut locals = param_types.clone();
    let params: Vec<Local> = (0..param_types.len() as Local).collect();
    let mut add = |t: Type| {
        locals.push(t);
        (locals.len() - 1) as Local
    };
    let (mark, value, error) = (add(Type::I64), add(f.ret), add(error_ty));
    let args = params
        .iter()
        .zip(&param_types)
        .map(|(p, t)| local(*p, *t, line))
        .collect();
    let call = at(K::Call(inner_id, args), f.ret, line);
    let leave = at(
        K::Rt(exit, vec![local(mark, Type::I64, line)]),
        Type::Unit,
        line,
    );
    let mut stmts = vec![Stmt::Let(
        mark,
        at(K::Rt(enter, arg.into_iter().collect()), Type::I64, line),
    )];
    if f.fails {
        stmts.push(Stmt::LetResult { value, error, call });
        stmts.push(Stmt::Expr(leave));
        let fail = at(
            K::Fail(Box::new(local(error, error_ty, line))),
            Type::Unit,
            line,
        );
        let check = K::If(
            Box::new(at(
                K::IsSome(Box::new(local(error, error_ty, line))),
                Type::Bool,
                line,
            )),
            Block {
                stmts: vec![Stmt::Expr(fail)],
                value: None,
            },
            Block::default(),
        );
        stmts.push(Stmt::Expr(at(check, Type::Unit, line)));
    } else if f.ret == Type::Unit {
        stmts.push(Stmt::Expr(call));
        stmts.push(Stmt::Expr(leave));
    } else {
        stmts.push(Stmt::Let(value, call));
        stmts.push(Stmt::Expr(leave));
    }
    let result = (f.ret != Type::Unit).then(|| Box::new(local(value, f.ret, line)));
    f.params = params;
    f.locals = locals;
    f.body = Block {
        stmts,
        value: result,
    };
    functions.push(inner);
}
