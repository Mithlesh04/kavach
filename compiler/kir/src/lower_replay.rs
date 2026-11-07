//! Events run again after a restart, in native code (spec 8.3, decision
//! 0072). `start` runs each in a task of its own and waits for it until
//! it ends or waits for a person; then `start` returns and the event goes
//! on, as a task of `start`'s caller. A send of the same event joins it.
//! The runtime half is `runtime/native/src/rt_replays.rs`.

use crate::ir::{self, Block, ExprKind as K, FuncId, Local, Shape, Stmt, Type};
use crate::lower::Lowerer;
use crate::lower_agents::{expr, int, rt, when};
use crate::rt_op::RtOp;

impl<'a> Lowerer<'a> {
    /// Runs event function `event` again, with `values` (the agent first,
    /// then the arguments), as a task, and waits for it until it ends or
    /// waits for a person.
    pub(crate) fn replay_task(
        &mut self,
        event: FuncId,
        values: Vec<ir::Expr>,
        ret: Type,
        err: Option<Type>,
        line: u32,
    ) -> Vec<Stmt> {
        let types: Vec<Type> = values.iter().map(|v| v.ty).collect();
        let handle = expr(K::GetField(Box::new(values[0].clone()), 0), Type::I64, line);
        let env_ty = self.shapes_intern(Shape::Closure(types.clone()));
        let saved = self.save_function(ret);
        let body = self.replay_body(event, env_ty, &types, ret, err, line);
        self.restore_function(saved);
        let body = self.add_function(body);
        let saved = self.save_function(Type::Unit);
        let error_ty = self.error_type();
        let thunk = self.task_thunk(body, env_ty, ret, error_ty, line);
        self.restore_function(saved);
        let thunk = self.add_function(thunk);
        let func_ty = self.shapes_intern(Shape::Func(Vec::new(), Type::Unit));
        let closure = K::Closure {
            func: thunk,
            env: env_ty,
            captures: values,
        };
        let task_ty = self.shapes_intern(Shape::Task(ret));
        let task = self.new_local(task_ty);
        let start = vec![expr(closure, func_ty, line), handle.clone()];
        let wait = vec![handle, self.local_expr(task, line)];
        vec![
            Stmt::Let(task, rt(RtOp::AgentReplay, start, task_ty, line)),
            Stmt::Expr(rt(RtOp::AgentReplayWait, wait, Type::Unit, line)),
        ]
    }

    /// The task's body: calls the event, tells the runtime how it ended
    /// (for the audit log), and passes its value or error on.
    fn replay_body(
        &mut self,
        event: FuncId,
        env_ty: Type,
        types: &[Type],
        ret: Type,
        err: Option<Type>,
        line: u32,
    ) -> ir::Function {
        let env = self.new_local(env_ty);
        let field = |l: &Self, i: usize, t: Type| {
            expr(
                K::GetField(Box::new(l.local_expr(env, line)), i as u32 + 1),
                t,
                line,
            )
        };
        let args: Vec<ir::Expr> = (types.iter().enumerate())
            .map(|(i, t)| field(self, i, *t))
            .collect();
        let handle = expr(
            K::GetField(Box::new(field(self, 0, types[0])), 0),
            Type::I64,
            line,
        );
        let call = expr(K::Call(event, args), ret, line);
        let value = self.new_local(ret);
        let mut stmts = Vec::new();
        match err {
            Some(e) => {
                let error = self.new_local(e);
                stmts.push(Stmt::LetResult { value, error, call });
                let failed = self.local_expr(error, line);
                let fail = expr(K::Fail(Box::new(failed.clone())), Type::Unit, line);
                let then = vec![
                    self.replay_end(handle.clone(), false, failed, line),
                    Stmt::Expr(fail),
                ];
                stmts.push(when(self.failed(error, line), then, line));
            }
            None => stmts.push(Stmt::Let(value, call)),
        }
        let ended = self.local_expr(value, line);
        stmts.push(self.replay_end(handle, true, ended, line));
        let result = (ret != Type::Unit).then(|| Box::new(self.local_expr(value, line)));
        ir::Function {
            name: "replayed event".into(),
            params: vec![env],
            ret,
            locals: self.locals.clone(),
            body: Block {
                stmts,
                value: result,
            },
            line,
            fails: true,
            external: None,
        }
    }

    fn replay_end(&mut self, handle: ir::Expr, ok: bool, outcome: ir::Expr, line: u32) -> Stmt {
        let tuple = self.tuple_type(vec![outcome.ty]);
        let ops = vec![
            handle,
            int(if ok { 1 } else { 2 }, line),
            expr(K::Tuple(vec![outcome]), tuple, line),
        ];
        Stmt::Expr(rt(RtOp::AgentReplayEnd, ops, Type::Unit, line))
    }

    /// At the start of a send of a durable agent's handler `name`: a
    /// replayed event of it with equal arguments that still waits for a
    /// person is joined: the send waits for its task and gives its outcome.
    pub(crate) fn join_replay(
        &mut self,
        a: Local,
        name: &str,
        values: Vec<ir::Expr>,
        ret: Type,
        line: u32,
    ) -> Stmt {
        let text = self.str_type();
        let tuple = self.tuple_type(values.iter().map(|v| v.ty).collect());
        let handle = expr(
            K::GetField(Box::new(self.local_expr(a, line)), 0),
            Type::I64,
            line,
        );
        let ask = vec![
            handle,
            expr(K::Str(name.into()), text, line),
            expr(K::Tuple(values), tuple, line),
        ];
        let joining = rt(RtOp::AgentJoining, ask, Type::Bool, line);
        let task_ty = self.shapes_intern(Shape::Task(ret));
        let task = rt(RtOp::AgentJoined, vec![int(0, line)], task_ty, line);
        let error_ty = self.error_type();
        let (value, error) = (self.new_local(ret), self.new_local(error_ty));
        let call = rt(RtOp::TaskWait, vec![task], ret, line);
        let mut then = vec![Stmt::LetResult { value, error, call }];
        // The task's error is the send's: its own, a panic, or that it was
        // cancelled.
        let fail = K::Fail(Box::new(self.local_expr(error, line)));
        let fail = vec![Stmt::Expr(expr(fail, Type::Unit, line))];
        then.push(when(self.failed(error, line), fail, line));
        let back = (ret != Type::Unit).then(|| self.local_expr(value, line));
        then.push(Stmt::Return(back));
        when(joining, then, line)
    }
}
