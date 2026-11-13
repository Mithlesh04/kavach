//! Function values that can fail, in native code (spec 3.4, 5.2,
//! decision 0085). A function type that can fail is a `Shape::Func` with
//! an error type, and its closures return a value and an error, as a
//! failing function does: a call through one is a `LetResult`.
//!
//! - A function that can't fail, given where one that can is wanted, is
//!   wrapped in a closure that calls it and never fails.
//! - A built-in loop (`map`, `filter`, …) given a function that can fail
//!   calls it as a closure, and its first error ends the loop. That error
//!   is the loop's, which `try` or `catch` around the call then reads
//!   (`pending_error`, taken by `outcome`).

use crate::ir::{self, Block, ExprKind as K, FuncId, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// The error type the functions of function type `ty` can fail with.
    pub(crate) fn fn_error(&self, ty: Type) -> Option<Type> {
        match self.shapes.shape(ty) {
            Some(Shape::Func(_, _, err)) => *err,
            _ => None,
        }
    }

    /// The body of a function that can fail and ends with `call`, which
    /// can fail with `own`: its error is passed on, as `try` does.
    pub(crate) fn passing_on(&mut self, call: ir::Expr, own: Type) -> Block {
        let (line, ret) = (call.line, call.ty);
        let err = self.held_error(own);
        let (value, error) = (self.new_local(ret), self.new_local(err));
        let fail = ir::Expr {
            kind: K::Fail(Box::new(self.local_expr(error, line))),
            ty: Type::Unit,
            line,
        };
        let check = ir::Expr {
            kind: K::If(
                Box::new(self.failed(error, line)),
                Block {
                    stmts: vec![Stmt::Expr(fail)],
                    value: None,
                },
                Block::default(),
            ),
            ty: Type::Unit,
            line,
        };
        Block {
            stmts: vec![Stmt::LetResult { value, error, call }, Stmt::Expr(check)],
            value: (ret != Type::Unit).then(|| Box::new(self.local_expr(value, line))),
        }
    }

    /// Function value `inner`, which can't fail, as function type `want`,
    /// which can: a closure around it that never fails.
    pub(crate) fn never_failing(&mut self, inner: ir::Expr, want: Type) -> Lowered<ir::Expr> {
        let line = inner.line;
        let Some(func) = self.adapter(inner.ty, want, line) else {
            return Ok(inner);
        };
        let env_ty = self.shapes_intern(Shape::Closure(vec![inner.ty]));
        Ok(ir::Expr {
            kind: K::Closure {
                func,
                env: env_ty,
                captures: vec![inner],
            },
            ty: want,
            line,
        })
    }

    /// The closure function that calls a value of type `inner`, which
    /// can't fail, as one of type `want`, which can: made once per pair,
    /// so `json.describe` can see through it (spec 2.12).
    pub(crate) fn adapter(&mut self, inner: Type, want: Type, line: u32) -> Option<FuncId> {
        if let Some(f) = self.adapters.get(&(inner, want)) {
            return Some(*f);
        }
        let Some(Shape::Func(param_types, ret, _)) = self.shapes.shape(want).cloned() else {
            return None;
        };
        let env_ty = self.shapes_intern(Shape::Closure(vec![inner]));
        let saved = self.save_function(ret);
        let env = self.new_local(env_ty);
        let params: Vec<Local> = param_types.iter().map(|t| self.new_local(*t)).collect();
        let read = ir::Expr {
            kind: K::GetField(Box::new(self.local_expr(env, line)), 1),
            ty: inner,
            line,
        };
        let call = ir::Expr {
            kind: K::CallValue(
                Box::new(read),
                params.iter().map(|p| self.local_expr(*p, line)).collect(),
            ),
            ty: ret,
            line,
        };
        let body = match ret {
            Type::Unit => Block {
                stmts: vec![Stmt::Expr(call)],
                value: None,
            },
            _ => Block {
                stmts: Vec::new(),
                value: Some(Box::new(call)),
            },
        };
        let mut all = vec![env];
        all.extend(params);
        let f = ir::Function {
            name: "function value that never fails".into(),
            params: all,
            ret,
            locals: self.locals.clone(),
            body,
            line,
            fails: true,
            external: None,
        };
        self.restore_function(saved);
        let func = self.add_function(f);
        self.adapters.insert((inner, want), func);
        Some(func)
    }

    /// For a built-in loop over `item` with function `f` that can fail:
    /// the statements that set up the loop (`f` as a closure, and the
    /// loop's error, none yet), the statements at the top of each round
    /// (the call, and leaving the loop at its first error), and the call's
    /// value. `None` when `f` can't fail.
    #[allow(clippy::type_complexity)]
    pub(crate) fn failing_rounds(
        &mut self,
        f: &'a ast::Expr,
        item: ir::Expr,
    ) -> Lowered<Option<(Vec<Stmt>, Vec<Stmt>, ir::Expr)>> {
        let fn_ty = self.ty_of(f)?;
        let Some(own) = self.fn_error(fn_ty) else {
            return Ok(None);
        };
        let line = item.line;
        let (params, ret) = self.fn_parts(fn_ty, f.span)?;
        let err = self.held_error(own);
        let closure = self.expr(f)?;
        let (held, loop_error) = (self.new_local(fn_ty), self.new_local(err));
        let none = ir::Expr {
            kind: K::None,
            ty: err,
            line,
        };
        let setup = vec![Stmt::Let(held, closure), Stmt::Let(loop_error, none)];
        let item = match params.first() {
            Some(t) => self.coerce(item, *t)?,
            None => item,
        };
        let call = ir::Expr {
            kind: K::CallValue(Box::new(self.local_expr(held, line)), vec![item]),
            ty: ret,
            line,
        };
        let (value, error) = (self.new_local(ret), self.new_local(err));
        let keep = ir::Expr {
            kind: K::Assign(loop_error, Box::new(self.local_expr(error, line))),
            ty: Type::Unit,
            line,
        };
        let leave = ir::Expr {
            kind: K::If(
                Box::new(self.failed(error, line)),
                Block {
                    stmts: vec![Stmt::Expr(keep), Stmt::Break],
                    value: None,
                },
                Block::default(),
            ),
            ty: Type::Unit,
            line,
        };
        let round = vec![Stmt::LetResult { value, error, call }, Stmt::Expr(leave)];
        self.pending_error = Some(loop_error);
        Ok(Some((setup, round, self.local_expr(value, line))))
    }

    /// `s.update(f)` and `s.update_with(f)` (spec 6.2) with an `f` that
    /// can fail: `f` is called with the value locked, and its error leaves
    /// the value as it was and is the call's (spec 5.2). `None` when `f`
    /// can't fail.
    pub(crate) fn failing_update(
        &mut self,
        shared: ir::Expr,
        f: &'a ast::Expr,
        with: bool,
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let fn_ty = self.ty_of(f)?;
        let Some(own) = self.fn_error(fn_ty) else {
            return Ok(None);
        };
        let Some(Shape::Shared(inner)) = self.shapes.shape(shared.ty).cloned() else {
            return Err(self.unsupported("`update` here", f.span));
        };
        let (params, ret) = self.fn_parts(fn_ty, f.span)?;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let err = self.held_error(own);
        let (local, held) = (self.new_local(shared.ty), self.new_local(fn_ty));
        let closure = self.expr(f)?;
        let rt = |l: &Self, op, mut args: Vec<ir::Expr>, ty| {
            args.insert(0, l.local_expr(local, line));
            at(K::Rt(op, args), ty)
        };
        let mut current = rt(self, RtOp::SharedGet, Vec::new(), inner);
        if let Some(t) = params.first() {
            current = self.coerce(current, *t)?;
        }
        let call = at(
            K::CallValue(Box::new(self.local_expr(held, line)), vec![current]),
            ret,
        );
        let (value, error) = (self.new_local(ret), self.new_local(err));
        let result = self.new_local(ty);
        // What `f` gave becomes the value, while the lock is still held.
        let (new, out) = if with {
            let Some(Shape::Tuple(parts)) = self.shapes.shape(ret).cloned() else {
                return Err(self.unsupported("`update_with` here", f.span));
            };
            let new = self.tuple_item(value, ret, 0, parts[0], line);
            (new, self.tuple_item(value, ret, 1, parts[1], line))
        } else {
            let now = rt(self, RtOp::SharedGet, Vec::new(), inner);
            (self.local_expr(value, line), now)
        };
        let new = self.coerce(new, inner)?;
        let set = rt(self, RtOp::SharedSet, vec![new], Type::Unit);
        let out = self.coerce(out, ty)?;
        let keep = Block {
            stmts: vec![
                Stmt::Expr(set),
                Stmt::Expr(at(K::Assign(result, Box::new(out)), Type::Unit)),
            ],
            value: None,
        };
        let changed = at(
            K::If(
                Box::new(self.succeeded(error, line)),
                keep,
                Block::default(),
            ),
            Type::Unit,
        );
        let stmts = vec![
            Stmt::Let(local, shared),
            Stmt::Let(held, closure),
            Stmt::Let(result, self.zero_of(ty, line)),
            Stmt::Expr(rt(self, RtOp::SharedLock, Vec::new(), Type::Unit)),
            Stmt::LetResult { value, error, call },
            Stmt::Expr(changed),
            Stmt::Expr(rt(self, RtOp::SharedUnlock, Vec::new(), Type::Unit)),
        ];
        self.pending_error = Some(error);
        let value = Some(Box::new(self.local_expr(result, line)));
        Ok(Some(at(K::Block(Block { stmts, value }), ty)))
    }

    /// The name of method call `e`, when it is given a function that can
    /// fail and its error is not one native code passes on yet.
    pub(crate) fn unhandled_failing_method(&self, e: &ast::Expr) -> Option<String> {
        let ast::ExprKind::Call { callee, args, .. } = &e.kind else {
            return None;
        };
        let ast::ExprKind::Field { base, name, .. } = &callee.kind else {
            return None;
        };
        // A method of a program's own type calls the function as any
        // function value is called, so its error is handled there.
        let base_ty = self.c.expr_types.get(&(base.span.start, base.span.end));
        if matches!(base_ty, Some(kavach_checker::Ty::Adt(..))) {
            return None;
        }
        let fails = |a: &ast::Arg| {
            let key = (a.value.span.start, a.value.span.end);
            matches!(
                self.c.expr_types.get(&key),
                Some(kavach_checker::Ty::Fn(_, r, _)) if matches!(**r, kavach_checker::Ty::Result(..))
            )
        };
        args.iter().any(fails).then(|| name.name.clone())
    }
}
