//! Lowering one function (split from `lower.rs`): its parameters, with
//! `self` and the types of a generic instance, its body, and the checks
//! its contracts make.

use crate::ir::{self, Stmt, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn function(
        &mut self,
        f: &'a ast::FnDecl,
        owner: Option<Owner>,
    ) -> Lowered<ir::Function> {
        if let Some(library) = &f.c_library {
            return self.c_function(f, library);
        }
        let ret = match &f.ret {
            Some(t) => self.written_type(t)?,
            None => Type::Unit,
        };
        self.start_function(ret);
        let mut params = Vec::new();
        for p in &f.params {
            let ty = match (&p.ty, &owner) {
                (Some(t), _) => self.written_type(t)?,
                (None, Some(owner)) => self.owner_type(owner, p.name.span)?,
                (None, None) => return Err(self.unsupported("`self` outside a type", p.name.span)),
            };
            let local = self.new_local(ty);
            if p.ty.is_none() {
                self.self_local = Some(local);
            }
            self.bind(&p.name, local);
            params.push(local);
        }
        let mut body = ir::Block::default();
        for r in &f.requires {
            let cond = self.expr(r)?;
            body.stmts.push(Stmt::Check {
                cond,
                message: format!("contract failed: require {}", self.text(r.span)),
                line: self.line(r.span),
            });
        }
        let Some(ast_body) = &f.body else {
            return Err(self.unsupported("functions without a body", f.name.span));
        };
        let line = self.line(ast_body.span);
        if f.ensures.is_empty() {
            self.block_into(ast_body, ret != Type::Unit, &mut body)?;
            if let Some(v) = body.value.take() {
                let v = self.coerce(*v, ret)?;
                body.value = Some(Box::new(v));
            }
        } else {
            let result = self.new_local(ret);
            self.result = Some(result);
            self.ensures = &f.ensures;
            let mut inner = ir::Block::default();
            self.block_into(ast_body, ret != Type::Unit, &mut inner)?;
            if let Some(v) = inner.value.take() {
                let v = self.coerce(*v, ret)?;
                inner.value = Some(Box::new(v));
            }
            body.stmts.push(Stmt::Let(
                result,
                ir::Expr {
                    kind: ir::ExprKind::Block(inner),
                    ty: ret,
                    line,
                },
            ));
            self.ensure_checks(&mut body.stmts)?;
            body.value = Some(Box::new(self.local_expr(result, line)));
        }
        let name = match &owner {
            Some(_) => format!("method {}", f.name.name),
            None => f.name.name.clone(),
        };
        // `inout` parameters go back with the value (spec 3.9).
        let inout: Vec<ir::Local> = f
            .params
            .iter()
            .zip(&params)
            .filter(|(p, _)| p.is_inout)
            .map(|(_, l)| *l)
            .collect();
        let ret = if inout.is_empty() {
            ret
        } else {
            let types: Vec<Type> = inout.iter().map(|l| self.locals[*l as usize]).collect();
            let tuple = self.inout_tuple(ret, &types);
            self.give_inout_back(&mut body, &inout, ret, tuple, line);
            tuple
        };
        Ok(ir::Function {
            name,
            params,
            ret,
            locals: self.locals.clone(),
            body,
            line: self.line(f.name.span),
            fails: f.errors.is_some(),
            external: None,
        })
    }
}
