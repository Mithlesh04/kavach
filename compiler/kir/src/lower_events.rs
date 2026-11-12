//! A send to an agent (`bot.request(order)`, spec 8.8, decision 0076):
//! the event's function (`event_fn` in `lower_agents.rs`, which pushes the
//! agent, journals the event and calls the handler) runs through
//! `AgentRun`, so a panic in the handler stops only the handler: the send
//! fails with `panicked: …`. `AgentRun` runs it at once, in the sender's
//! task, through a thunk that hands its value or error over as a task's
//! does (`runtime/native/src/rt_events.rs`). Every send can fail.

use crate::ir::{self, Block, ExprKind as K, FuncId, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_agents::{expr, rt, when};
use crate::rt_op::RtOp;
use kavach_resolver::DefId;

impl Lowerer<'_> {
    /// The function a send of handler `name` calls: the same parameters
    /// as the event's function `event` of agent `agent` (the agent, then
    /// the handler's), giving `ret`, through `AgentRun`. It can always
    /// fail. In a durable agent, it first joins the same event run again
    /// after a restart, if that waits for a person.
    pub(crate) fn event_send(
        &mut self,
        event: FuncId,
        agent: DefId,
        types: &[Type],
        (ret, event_fails): (Type, bool),
        name: &str,
        line: u32,
    ) -> Lowered<ir::Function> {
        let env_ty = self.shapes_intern(Shape::Closure(types.to_vec()));
        let thunk = self.event_thunk(event, env_ty, types, ret, event_fails, line);
        let thunk = self.add_function(thunk);
        self.start_function(ret);
        let params: Vec<ir::Local> = types.iter().map(|t| self.new_local(*t)).collect();
        let captures = params.iter().map(|p| self.local_expr(*p, line)).collect();
        let joins = self.journal_of(agent).is_some().then(|| {
            let values = params[1..].iter().map(|p| self.local_expr(*p, line));
            let values = values.collect();
            self.join_replay(params[0], name, values, ret, line)
        });
        let func_ty = self.shapes_intern(Shape::Func(Vec::new(), Type::Unit, None));
        let closure = expr(
            K::Closure {
                func: thunk,
                env: env_ty,
                captures,
            },
            func_ty,
            line,
        );
        let text = self.str_type();
        // The agent's handle, which the runtime keeps it by: its first field.
        let agent = Box::new(self.local_expr(params[0], line));
        let args = vec![
            closure,
            expr(K::GetField(agent, 0), Type::I64, line),
            expr(K::Str(name.to_string()), text, line),
        ];
        let call = rt(RtOp::AgentRun, args, ret, line);
        let error_ty = self.error_type();
        let (value, error) = (self.new_local(ret), self.new_local(error_ty));
        let fail = expr(
            K::Fail(Box::new(self.local_expr(error, line))),
            Type::Unit,
            line,
        );
        let mut stmts: Vec<Stmt> = joins.into_iter().collect();
        stmts.push(Stmt::LetResult { value, error, call });
        stmts.push(when(self.failed(error, line), vec![Stmt::Expr(fail)], line));
        let body = Block {
            stmts,
            value: (ret != Type::Unit).then(|| Box::new(self.local_expr(value, line))),
        };
        Ok(ir::Function {
            name: format!("send {name}"),
            params,
            ret,
            locals: self.locals.clone(),
            body,
            line,
            fails: true,
            external: None,
        })
    }

    /// The thunk `AgentRun` runs: the event's function with the values the
    /// closure holds, its value or error handed over (`TaskDone`,
    /// `TaskFail`).
    fn event_thunk(
        &mut self,
        event: FuncId,
        env_ty: Type,
        types: &[Type],
        ret: Type,
        event_fails: bool,
        line: u32,
    ) -> ir::Function {
        self.start_function(Type::Unit);
        let env = self.new_local(env_ty);
        let args = (types.iter().enumerate())
            .map(|(i, t)| {
                let base = Box::new(self.local_expr(env, line));
                expr(K::GetField(base, i as u32 + 1), *t, line)
            })
            .collect();
        let call = expr(K::Call(event, args), ret, line);
        let value = self.new_local(ret);
        let done = match ret {
            Type::Unit => expr(K::Int(0), Type::I64, line),
            _ => self.local_expr(value, line),
        };
        let hand_over = |op, arg| Stmt::Expr(rt(op, vec![arg], Type::Unit, line));
        let mut stmts = Vec::new();
        if event_fails {
            let error_ty = self.error_type();
            let error = self.new_local(error_ty);
            stmts.push(Stmt::LetResult { value, error, call });
            let failed = self.failed(error, line);
            let fail = hand_over(RtOp::TaskFail, self.local_expr(error, line));
            let choice = K::If(
                Box::new(failed),
                Block {
                    stmts: vec![fail],
                    value: None,
                },
                Block {
                    stmts: vec![hand_over(RtOp::TaskDone, done)],
                    value: None,
                },
            );
            stmts.push(Stmt::Expr(expr(choice, Type::Unit, line)));
        } else {
            match ret {
                Type::Unit => stmts.push(Stmt::Expr(call)),
                _ => stmts.push(Stmt::Let(value, call)),
            }
            stmts.push(hand_over(RtOp::TaskDone, done));
        }
        ir::Function {
            name: "event thunk".into(),
            params: vec![env],
            ret: Type::Unit,
            locals: self.locals.clone(),
            body: Block { stmts, value: None },
            line,
            fails: false,
            external: None,
        }
    }
}
