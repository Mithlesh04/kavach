//! `spawn process f(args)` (spec 9.8, decision 0075). In the parent it is
//! a task whose thunk runs the call in a child (`ProcessRun`, which can
//! fail) and hands its value or error over, as any task's thunk does. In
//! the child, each such call has an entry: the arguments it was given, the
//! call, and its value or error given back. The program's start runs the
//! entry instead of the program when it is a child (`process_starts`).
//! The runtime's side is `runtime/native/src/rt_process.rs`.

use crate::ir::{self, Block, ExprKind as K, Function, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_checker::Ty;

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

fn local(l: ir::Local, ty: Type, line: u32) -> ir::Expr {
    at(K::Local(l), ty, line)
}

fn stmt(op: RtOp, args: Vec<ir::Expr>, line: u32) -> Stmt {
    Stmt::Expr(at(K::Rt(op, args), Type::Unit, line))
}

impl<'a> Lowerer<'a> {
    /// `spawn process x`, of task type `ty`.
    pub(crate) fn spawn_process(
        &mut self,
        x: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Shape::Task(value_ty)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("this `spawn process`", x.span));
        };
        let ast::ExprKind::Call { callee, args, .. } = &x.kind else {
            return Err(self.unsupported("`spawn process` of a block", x.span));
        };
        let fid = match &callee.kind {
            ast::ExprKind::Name(id) => (self.c.resolution.uses.get(&id.span.start))
                .and_then(|d| self.fn_ids.get(d))
                .copied(),
            _ => None,
        };
        let Some(fid) = fid else {
            return Err(self.unsupported("this `spawn process` call", x.span));
        };
        let values = self.arguments(self.decl_of(fid), args, x.span)?;
        let types: Vec<Type> = values.iter().map(|v| v.ty).collect();
        let args_ty = self.tuple_type(types);
        let fails = matches!(
            self.c.expr_types.get(&(x.span.start, x.span.end)),
            Some(Ty::Result(..))
        );
        let site = self.process_entries.len() as i128;
        let saved = self.save_function(Type::Unit);
        let entry = self.process_entry(fid, args_ty, value_ty, fails, line);
        self.restore_function(saved);
        let entry = self.add_function(entry);
        self.process_entries.push(entry);

        let env_ty = self.shapes_intern(Shape::Closure(vec![args_ty]));
        let saved = self.save_function(Type::Unit);
        let error_ty = self.error_type();
        let thunk = self.process_thunk(site, env_ty, args_ty, value_ty, error_ty, line);
        self.restore_function(saved);
        let thunk = self.add_function(thunk);
        let func_ty = self.shapes_intern(Shape::Func(Vec::new(), Type::Unit, None));
        let closure = K::Closure {
            func: thunk,
            env: env_ty,
            captures: vec![at(K::Tuple(values), args_ty, line)],
        };
        let spawn = vec![at(closure, func_ty, line), at(K::Int(0), Type::I64, line)];
        Ok(at(K::Rt(RtOp::TaskSpawn, spawn), ty, line))
    }

    /// The parent's task: `ProcessRun(site, args)`, then its value or
    /// error handed over.
    fn process_thunk(
        &mut self,
        site: i128,
        env_ty: Type,
        args_ty: Type,
        value_ty: Type,
        error_ty: Type,
        line: u32,
    ) -> Function {
        let env = self.new_local(env_ty);
        let (value, error) = (self.new_local(value_ty), self.new_local(error_ty));
        let args = at(
            K::GetField(Box::new(local(env, env_ty, line)), 1),
            args_ty,
            line,
        );
        let call = at(
            K::Rt(
                RtOp::ProcessRun,
                vec![at(K::Int(site), Type::I64, line), args],
            ),
            value_ty,
            line,
        );
        let done = match value_ty {
            Type::Unit => at(K::Int(0), Type::I64, line),
            _ => local(value, value_ty, line),
        };
        let failed = at(
            K::IsSome(Box::new(local(error, error_ty, line))),
            Type::Bool,
            line,
        );
        let choice = K::If(
            Box::new(failed),
            Block {
                stmts: vec![stmt(
                    RtOp::TaskFail,
                    vec![local(error, error_ty, line)],
                    line,
                )],
                value: None,
            },
            Block {
                stmts: vec![stmt(RtOp::TaskDone, vec![done], line)],
                value: None,
            },
        );
        Function {
            name: "process task thunk".into(),
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

    /// The child's entry for a call of `fid`: its arguments, the call, and
    /// its value (in a one-item tuple) or error given back.
    fn process_entry(
        &mut self,
        fid: ir::FuncId,
        args_ty: Type,
        value_ty: Type,
        fails: bool,
        line: u32,
    ) -> Function {
        let types = match self.shapes.shape(args_ty) {
            Some(Shape::Tuple(ts)) => ts.clone(),
            _ => Vec::new(),
        };
        let args = self.new_local(args_ty);
        let given = (0..types.len() as u32)
            .map(|i| {
                at(
                    K::GetField(Box::new(local(args, args_ty, line)), i),
                    types[i as usize],
                    line,
                )
            })
            .collect();
        let call = at(K::Call(fid, given), value_ty, line);
        let mut stmts = vec![Stmt::Let(
            args,
            at(K::Rt(RtOp::ProcessArgs, Vec::new()), args_ty, line),
        )];
        let value = self.new_local(value_ty);
        let give_back = match value_ty {
            Type::Unit => at(K::Int(0), Type::I64, line),
            _ => {
                let one = self.tuple_type(vec![value_ty]);
                at(K::Tuple(vec![local(value, value_ty, line)]), one, line)
            }
        };
        let done = stmt(RtOp::ProcessDone, vec![give_back], line);
        if fails {
            let error_ty = self.error_type();
            let error = self.new_local(error_ty);
            stmts.push(Stmt::LetResult { value, error, call });
            let failed = at(
                K::IsSome(Box::new(local(error, error_ty, line))),
                Type::Bool,
                line,
            );
            let choice = K::If(
                Box::new(failed),
                Block {
                    stmts: vec![stmt(
                        RtOp::ProcessFailed,
                        vec![local(error, error_ty, line)],
                        line,
                    )],
                    value: None,
                },
                Block {
                    stmts: vec![done],
                    value: None,
                },
            );
            stmts.push(Stmt::Expr(at(choice, Type::Unit, line)));
        } else {
            match value_ty {
                Type::Unit => stmts.push(Stmt::Expr(call)),
                _ => stmts.push(Stmt::Let(value, call)),
            }
            stmts.push(done);
        }
        Function {
            name: "process entry".into(),
            params: Vec::new(),
            ret: Type::Unit,
            locals: self.locals.clone(),
            body: Block { stmts, value: None },
            line,
            fails: false,
            external: None,
        }
    }

    /// What the program's start does in a child: `if ProcessSite() == k
    /// { entry_k(); return }` for each call it may be told to run.
    pub(crate) fn process_starts(&self) -> Vec<Stmt> {
        let line = 1;
        (self.process_entries.iter().enumerate())
            .map(|(k, &entry)| {
                let site = at(K::Rt(RtOp::ProcessSite, Vec::new()), Type::I64, line);
                let this = at(
                    K::Binary(
                        ir::BinOp::Eq,
                        Box::new(site),
                        Box::new(at(K::Int(k as i128), Type::I64, line)),
                    ),
                    Type::Bool,
                    line,
                );
                let run = Block {
                    stmts: vec![
                        Stmt::Expr(at(K::Call(entry, Vec::new()), Type::Unit, line)),
                        Stmt::Return(None),
                    ],
                    value: None,
                };
                Stmt::Expr(at(
                    K::If(Box::new(this), run, Block::default()),
                    Type::Unit,
                    line,
                ))
            })
            .collect()
    }
}
