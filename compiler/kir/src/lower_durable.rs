//! Durable agents in native code (spec 8.5, decision 0025), for agents
//! with a `journal` in `kavach.toml`: `start` opens the journal and runs
//! again the events a crash left unfinished; each event and each guarded
//! call goes to the journal; a call the journal already has gives its
//! recorded outcome without running. The journal itself is the runtime's
//! (`runtime/native/src/rt_journal.rs`), shared with the interpreter.

use crate::ir::{self, BinOp, Block, ExprKind as K, FuncId, Stmt, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::lower_agents::{Synth, expr, int, rt, when};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, AgentMember};
use kavach_resolver::DefId;

/// The guard's handle of the agent in local `a`.
fn handle(l: &Lowerer<'_>, a: ir::Local, line: u32) -> ir::Expr {
    expr(
        K::GetField(Box::new(l.local_expr(a, line)), 0),
        Type::I64,
        line,
    )
}

fn equals(e: ir::Expr, n: i128, line: u32) -> ir::Expr {
    let n = expr(K::Int(n), Type::I64, line);
    expr(
        K::Binary(BinOp::Eq, Box::new(e), Box::new(n)),
        Type::Bool,
        line,
    )
}

impl<'a> Lowerer<'a> {
    /// The journal file of agent `def` (its settings, worked out when the
    /// program is compiled, decision 0060), if it is durable.
    pub(crate) fn journal_of(&self, def: DefId) -> Option<String> {
        let name = &self.agent_decl(def).name.name;
        self.c.agent_config(name, false).journal
    }

    /// `start` of a durable agent: the agent `started`, its journal
    /// opened, and its unfinished events run again before `start` returns.
    pub(crate) fn durable_start(
        &mut self,
        def: DefId,
        started: ir::Expr,
        journal: String,
        line: u32,
    ) -> ir::Expr {
        let ty = started.ty;
        let a = self.new_local(ty);
        let found = self.agents.synth.iter().find_map(|(f, s)| match s {
            Synth::Resume { agent } if *agent == def => Some(*f),
            _ => None,
        });
        let resume = found.unwrap_or_else(|| {
            let fid = self.fns.len() as FuncId;
            self.fns.push((None, None));
            self.agents.synth.insert(fid, Synth::Resume { agent: def });
            fid
        });
        let text = self.str_type();
        let open = vec![handle(self, a, line), expr(K::Str(journal), text, line)];
        let again = K::Call(resume, vec![self.local_expr(a, line)]);
        let stmts = vec![
            Stmt::Let(a, started),
            Stmt::Expr(rt(RtOp::JournalOpen, open, Type::Unit, line)),
            Stmt::Expr(expr(again, Type::Unit, line)),
        ];
        let value = Some(Box::new(self.local_expr(a, line)));
        expr(K::Block(Block { stmts, value }), ty, line)
    }

    /// The function that runs again, in order, the events agent `def`
    /// left unfinished, each with its arguments read back.
    pub(crate) fn resume_fn(&mut self, def: DefId) -> Lowered<ir::Function> {
        let decl = self.agent_decl(def);
        let span = decl.name.span;
        let agent_ty = self.agent_type(def, span)?;
        self.start_function(Type::Unit);
        let line = self.line(span);
        let a = self.new_local(agent_ty);
        let handlers: Vec<&'a ast::FnDecl> = decl
            .members
            .iter()
            .filter_map(|m| match m {
                AgentMember::On(f) => Some(f),
                _ => None,
            })
            .collect();
        let names: Vec<&str> = handlers.iter().map(|f| f.name.name.as_str()).collect();
        let text = self.str_type();
        let names = expr(K::Str(names.join(" ")), text, line);
        let next = rt(
            RtOp::JournalNext,
            vec![handle(self, a, line), names],
            Type::I64,
            line,
        );
        let h = self.new_local(Type::I64);
        let mut body = Vec::new();
        for (k, f) in handlers.into_iter().enumerate() {
            let event = self.method_ids[&(Owner::Adt(def), f.name.name.clone())];
            let mut then = Vec::new();
            let mut values = vec![self.local_expr(a, line)];
            for (i, p) in f.params.iter().enumerate() {
                let Some(t) = &p.ty else {
                    return Err(self.unsupported("`self` in a handler", p.name.span));
                };
                let ty = self.written_type(t)?;
                let templates = self.templates(ty, line, p.name.span)?;
                let x = self.new_local(ty);
                let arg = vec![handle(self, a, line), int(i, line), templates];
                then.push(Stmt::Let(x, rt(RtOp::JournalArg, arg, ty, line)));
                values.push(self.local_expr(x, line));
            }
            let ret = match &f.ret {
                Some(t) => self.written_type(t)?,
                None => Type::Unit,
            };
            // A task of its own, which `start` waits for until it ends or
            // waits for a person (decision 0072). Like any send, it can
            // fail: with the handler's error, or its panic (decision 0076).
            let err = Some(self.error_type());
            then.extend(self.replay_task(event, values, ret, err, line));
            body.push(when(
                equals(self.local_expr(h, line), k as i128, line),
                then,
                line,
            ));
        }
        body.push(Stmt::Expr(expr(
            K::Assign(h, Box::new(next.clone())),
            Type::Unit,
            line,
        )));
        let more = expr(
            K::Binary(
                BinOp::Ne,
                Box::new(self.local_expr(h, line)),
                Box::new(expr(K::Int(-1), Type::I64, line)),
            ),
            Type::Bool,
            line,
        );
        let stmts = vec![
            Stmt::Let(h, next),
            Stmt::While {
                cond: more,
                body: Block {
                    stmts: body,
                    value: None,
                },
            },
        ];
        Ok(ir::Function {
            name: format!("{}.start", self.agent_decl(def).name.name),
            params: vec![a],
            ret: Type::Unit,
            locals: self.locals.clone(),
            body: Block { stmts, value: None },
            line,
            fails: false,
            external: None,
        })
    }

    /// An event of handler `name` begins, with arguments `args`.
    pub(crate) fn journal_begin(
        &mut self,
        a: ir::Local,
        name: &str,
        args: Vec<ir::Expr>,
        line: u32,
    ) -> Stmt {
        let text = self.str_type();
        let tuple = self.tuple_type(args.iter().map(|e| e.ty).collect());
        let ops = vec![
            handle(self, a, line),
            expr(K::Str(name.into()), text, line),
            expr(K::Tuple(args), tuple, line),
        ];
        Stmt::Expr(rt(RtOp::JournalBegin, ops, Type::Unit, line))
    }

    /// The event ended.
    pub(crate) fn journal_end(&self, a: ir::Local, line: u32) -> Stmt {
        let ops = vec![handle(self, a, line)];
        Stmt::Expr(rt(RtOp::JournalEnd, ops, Type::Unit, line))
    }

    /// Before guarded call `call`: when the event runs again and the
    /// journal has the call's outcome, that value is returned or that
    /// error failed with, and the call doesn't run.
    pub(crate) fn journal_replay(
        &mut self,
        a: ir::Local,
        call: &str,
        (ret, err): (Type, Type),
        line: u32,
        span: ast::Span,
    ) -> Lowered<Vec<Stmt>> {
        let text = self.str_type();
        let found = self.new_local(Type::I64);
        let ask = vec![handle(self, a, line), expr(K::Str(call.into()), text, line)];
        let value = |l: &mut Self, ty: Type| -> Lowered<ir::Expr> {
            let templates = l.templates(ty, line, span)?;
            let ops = vec![handle(l, a, line), templates];
            Ok(rt(RtOp::JournalValue, ops, ty, line))
        };
        let back = match ret {
            Type::Unit => None,
            ty => Some(value(self, ty)?),
        };
        let fail = expr(K::Fail(Box::new(value(self, err)?)), Type::Unit, line);
        Ok(vec![
            Stmt::Let(found, rt(RtOp::JournalRecorded, ask, Type::I64, line)),
            when(
                equals(self.local_expr(found, line), 1, line),
                vec![Stmt::Return(back)],
                line,
            ),
            when(
                equals(self.local_expr(found, line), 2, line),
                vec![Stmt::Expr(fail)],
                line,
            ),
        ])
    }

    /// The guarded call ran: its value (`ok`) or error, in `outcome`.
    pub(crate) fn journal_record(
        &mut self,
        a: ir::Local,
        ok: bool,
        outcome: ir::Expr,
        line: u32,
    ) -> Stmt {
        let tuple = self.tuple_type(vec![outcome.ty]);
        let ops = vec![
            handle(self, a, line),
            int(if ok { 1 } else { 2 }, line),
            expr(K::Tuple(vec![outcome]), tuple, line),
        ];
        Stmt::Expr(rt(RtOp::JournalRecord, ops, Type::Unit, line))
    }
}
