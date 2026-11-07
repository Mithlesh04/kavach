//! Functions as values (spec 3.3, 3.4): lambdas that are stored, passed
//! or returned, and named functions used as values.
//!
//! Such a lambda becomes a function of its own, and its value is a
//! closure: a heap object holding the function's address and a copy of
//! each variable the lambda uses (taken when the lambda is created, like
//! the interpreter). The function gets the closure as its first
//! parameter and reads the copies from it. A lambda written straight into
//! `filter`, `map` and the like is still inlined (`lower_lambda.rs`).

use crate::ir::{self, ExprKind as K, FuncId, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use kavach_ast::{self as ast, ExprKind};
use kavach_resolver::DefId;
use std::collections::HashMap;

/// The state of the function being lowered, set aside while a lambda's
/// own function is lowered.
pub(crate) struct Saved {
    locals: Vec<Type>,
    local_ids: HashMap<DefId, Local>,
    ret: Type,
    result: Option<Local>,
    self_local: Option<Local>,
    its: Vec<Local>,
}

impl<'a> Lowerer<'a> {
    pub(crate) fn save_function(&mut self, ret: Type) -> Saved {
        let saved = Saved {
            locals: std::mem::take(&mut self.locals),
            local_ids: std::mem::take(&mut self.local_ids),
            ret: self.ret,
            result: self.result.take(),
            self_local: self.self_local.take(),
            its: std::mem::take(&mut self.its),
        };
        self.ret = ret;
        saved
    }

    pub(crate) fn restore_function(&mut self, s: Saved) {
        self.locals = s.locals;
        self.local_ids = s.local_ids;
        self.ret = s.ret;
        self.result = s.result;
        self.self_local = s.self_local;
        self.its = s.its;
    }

    /// Adds an already lowered function; returns its id.
    pub(crate) fn add_function(&mut self, f: ir::Function) -> FuncId {
        let id = self.fns.len() as FuncId;
        self.fns.push((None, None));
        self.ready.insert(id, f);
        id
    }

    /// The function type `ty` as (parameter types, return type).
    pub(crate) fn fn_parts(&self, ty: Type, span: ast::Span) -> Lowered<(Vec<Type>, Type)> {
        match self.shapes.shape(ty) {
            Some(Shape::Func(params, ret)) => Ok((params.clone(), *ret)),
            _ => Err(self.unsupported("this function value", span)),
        }
    }

    /// The variables of this function that the code at `f` uses, in
    /// order (`self` last, if it uses it): their definitions, locals and
    /// types.
    pub(crate) fn captures(&self, f: &ast::Expr) -> (Vec<DefId>, Vec<Local>, Vec<Type>) {
        let mut used: Vec<(u32, DefId)> = self
            .c
            .resolution
            .uses
            .iter()
            .filter(|(at, d)| {
                **at >= f.span.start && **at < f.span.end && self.local_ids.contains_key(d)
            })
            .map(|(at, d)| (*at, *d))
            .collect();
        used.sort_unstable();
        let mut captured: Vec<DefId> = Vec::new();
        for (_, d) in used {
            if !captured.contains(&d) {
                captured.push(d);
            }
        }
        let mut outer: Vec<Local> = captured.iter().map(|d| self.local_ids[d]).collect();
        // `self` inside a lambda in a method is captured last.
        let uses_self = self.self_local.is_some() && mentions_self(self.text(f.span));
        if let (true, Some(s)) = (uses_self, self.self_local) {
            outer.push(s);
        }
        let types: Vec<Type> = outer.iter().map(|l| self.locals[*l as usize]).collect();
        (captured, outer, types)
    }

    /// Lambda `f` (of function type `ty`) as a closure value.
    pub(crate) fn closure(&mut self, f: &'a ast::Expr, ty: Type, line: u32) -> Lowered<ir::Expr> {
        let (param_types, ret) = self.fn_parts(ty, f.span)?;
        let (captured, outer, types) = self.captures(f);
        let values: Vec<ir::Expr> = outer.iter().map(|l| self.local_expr(*l, line)).collect();
        let env_ty = self.shapes_intern(Shape::Closure(types.clone()));

        let saved = self.save_function(ret);
        let lowered = self.lambda_function(f, env_ty, &param_types, ret, &captured, &types, line);
        self.restore_function(saved);
        let func = self.add_function(lowered?);
        Ok(ir::Expr {
            kind: K::Closure {
                func,
                env: env_ty,
                captures: values,
            },
            ty,
            line,
        })
    }

    /// The function a lambda becomes: `(closure, params…) -> ret`.
    #[allow(clippy::too_many_arguments)]
    fn lambda_function(
        &mut self,
        f: &'a ast::Expr,
        env_ty: Type,
        param_types: &[Type],
        ret: Type,
        captured: &[DefId],
        types: &[Type],
        line: u32,
    ) -> Lowered<ir::Function> {
        let (env, mut body) = self.unpack_env(env_ty, captured, types, line);
        let mut params = vec![env];
        let value = match &f.kind {
            ExprKind::Lambda {
                params: ps,
                body: b,
            } => {
                for (p, t) in ps.iter().zip(param_types) {
                    let local = self.new_local(*t);
                    self.bind(&p.name, local);
                    params.push(local);
                }
                // Run for what it does: its last line is a statement.
                if ret == Type::Unit {
                    match &b.kind {
                        ExprKind::Block(blk) => self.block_into(blk, false, &mut body)?,
                        _ => self.expr_stmt(b, &mut body.stmts)?,
                    }
                    None
                } else {
                    Some(self.expr(b)?)
                }
            }
            ExprKind::DotLambda(b) => {
                let local = self.new_local(param_types[0]);
                params.push(local);
                self.its.push(local);
                Some(self.expr_as(b, ret)?)
            }
            _ => return Err(self.unsupported("this function value", f.span)),
        };
        match value {
            Some(v) if ret != Type::Unit => body.value = Some(Box::new(self.coerce(v, ret)?)),
            Some(v) => body.stmts.push(Stmt::Expr(v)),
            None => {}
        }
        Ok(ir::Function {
            name: "lambda".into(),
            params,
            ret,
            locals: self.locals.clone(),
            body,
            line,
            fails: false,
            external: None,
        })
    }

    /// A new local for a closure object of type `env_ty`, and a block
    /// that copies each captured value out of it into a local of its own.
    pub(crate) fn unpack_env(
        &mut self,
        env_ty: Type,
        captured: &[DefId],
        types: &[Type],
        line: u32,
    ) -> (Local, ir::Block) {
        let env = self.new_local(env_ty);
        let mut body = ir::Block::default();
        for (i, t) in types.iter().enumerate() {
            let local = self.new_local(*t);
            match captured.get(i) {
                Some(d) => {
                    self.local_ids.insert(*d, local);
                }
                None => self.self_local = Some(local),
            }
            let copy = ir::Expr {
                kind: K::GetField(Box::new(self.local_expr(env, line)), i as u32 + 1),
                ty: *t,
                line,
            };
            body.stmts.push(Stmt::Let(local, copy));
        }
        (env, body)
    }

    /// Named function `fid` used as a value: a closure around a small
    /// function that calls it.
    pub(crate) fn function_value(
        &mut self,
        fid: FuncId,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let (param_types, ret) = self.fn_parts(ty, span)?;
        let env_ty = self.shapes_intern(Shape::Closure(Vec::new()));
        let wrapper = match self.wrappers.get(&fid) {
            Some(w) => *w,
            None => {
                let saved = self.save_function(ret);
                let env = self.new_local(env_ty);
                let params: Vec<Local> = param_types.iter().map(|t| self.new_local(*t)).collect();
                let args = params.iter().map(|p| self.local_expr(*p, line)).collect();
                let call = ir::Expr {
                    kind: K::Call(fid, args),
                    ty: ret,
                    line,
                };
                let mut body = ir::Block::default();
                if ret == Type::Unit {
                    body.stmts.push(Stmt::Expr(call));
                } else {
                    body.value = Some(Box::new(call));
                }
                let mut all = vec![env];
                all.extend(params);
                let f = ir::Function {
                    name: "function value".into(),
                    params: all,
                    ret,
                    locals: self.locals.clone(),
                    body,
                    line,
                    fails: false,
                    external: None,
                };
                self.restore_function(saved);
                let w = self.add_function(f);
                self.wrappers.insert(fid, w);
                w
            }
        };
        Ok(ir::Expr {
            kind: K::Closure {
                func: wrapper,
                env: env_ty,
                captures: Vec::new(),
            },
            ty,
            line,
        })
    }

    /// Calls the function value `callee` with `args`.
    pub(crate) fn call_value(
        &mut self,
        callee: ir::Expr,
        args: Vec<ir::Expr>,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let (param_types, ret) = self.fn_parts(callee.ty, span)?;
        let args = args
            .into_iter()
            .zip(param_types)
            .map(|(a, t)| self.coerce(a, t))
            .collect::<Lowered<Vec<_>>>()?;
        Ok(ir::Expr {
            kind: K::CallValue(Box::new(callee), args),
            ty: ret,
            line,
        })
    }
}

/// Whether lambda text uses `self` (as a word, not inside a longer name).
fn mentions_self(text: &str) -> bool {
    text.match_indices("self").any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + 4..].chars().next();
        let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !word(before) && !word(after)
    })
}
