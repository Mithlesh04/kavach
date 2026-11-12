//! `inout` parameters in native code (spec 3.9, decision 0038): copied in
//! and copied back out. A function with `inout` parameters returns a
//! tuple, its own value first (when it has one), then each `inout`
//! parameter's last value; the caller stores those into the places its
//! arguments named. No pointer to a local is ever made.

use crate::ir::{self, Block, ExprKind as K, FuncId, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::walk::blocks_mut;
use kavach_ast as ast;

/// A call with `inout` parameters: their positions and names, the call's
/// arguments, the `inout` values' types and the call's own type.
pub(crate) struct Pending<'a> {
    inout: Vec<(usize, &'a str)>,
    args: &'a [ast::Arg],
    types: Vec<Type>,
    ty: Type,
}

impl<'a> Lowerer<'a> {
    /// The positions (among the parameters after `self`) and names of the
    /// `inout` parameters of `f`.
    pub(crate) fn inout_params(f: &'a ast::FnDecl) -> Vec<(usize, &'a str)> {
        f.params
            .iter()
            .filter(|p| p.name.name != "self")
            .enumerate()
            .filter(|(_, p)| p.is_inout)
            .map(|(i, p)| (i, p.name.name.as_str()))
            .collect()
    }

    /// The tuple a function returning `ret` with `inout` parameters of
    /// types `params` gives back.
    pub(crate) fn inout_tuple(&mut self, ret: Type, params: &[Type]) -> Type {
        let mut items: Vec<Type> = Vec::new();
        if ret != Type::Unit {
            items.push(ret);
        }
        items.extend(params);
        self.shapes_intern(Shape::Tuple(items))
    }

    /// Makes `body` (of a function returning `ret`) give the tuple
    /// `tuple`: its value and the `inout` locals, at its end and at every
    /// `return`.
    pub(crate) fn give_inout_back(
        &mut self,
        body: &mut Block,
        locals: &[Local],
        ret: Type,
        tuple: Type,
        line: u32,
    ) {
        let pack = |l: &Self, value: Option<ir::Expr>| {
            let mut items: Vec<ir::Expr> = value.into_iter().collect();
            items.extend(locals.iter().map(|x| l.local_expr(*x, line)));
            ir::Expr {
                kind: K::Tuple(items),
                ty: tuple,
                line,
            }
        };
        blocks_mut(body, &mut |b| {
            for s in &mut b.stmts {
                if let Stmt::Return(v) = s {
                    let value = v.take().filter(|_| ret != Type::Unit);
                    *v = Some(pack(self, value));
                }
            }
        });
        let value = body.value.take().map(|v| *v).filter(|_| ret != Type::Unit);
        body.value = Some(Box::new(pack(self, value)));
    }

    /// Call `call` of function `fid` (which returns its tuple), giving
    /// `ty`: after it, each `inout` value goes into the place its argument
    /// in `args` names. `offset` is 1 when the call's values start with a
    /// receiver.
    pub(crate) fn write_back(
        &mut self,
        fid: FuncId,
        call: ir::Expr,
        args: &'a [ast::Arg],
        offset: usize,
        ty: Type,
    ) -> Lowered<ir::Expr> {
        let decl = self.decl_of(fid);
        self.write_back_of(decl, call, args, offset, ty)
    }

    /// As `write_back`, for a call of the function or interface method
    /// declared as `decl`, whose values are `call`'s operands.
    pub(crate) fn write_back_of(
        &mut self,
        decl: &'a ast::FnDecl,
        call: ir::Expr,
        args: &'a [ast::Arg],
        offset: usize,
        ty: Type,
    ) -> Lowered<ir::Expr> {
        let inout = Self::inout_params(decl);
        if inout.is_empty() {
            return Ok(call);
        }
        let line = call.line;
        let (K::Call(_, values) | K::CallValue(_, values)) = &call.kind else {
            return Ok(call);
        };
        let types: Vec<Type> = inout.iter().map(|(i, _)| values[i + offset].ty).collect();
        let tuple = self.inout_tuple(ty, &types);
        let call = ir::Expr { ty: tuple, ..call };
        let p = Pending {
            inout,
            args,
            types,
            ty,
        };
        // A call that can fail gives its tuple to `outcome`, which writes
        // the values back only when it succeeds (spec 3.9).
        if decl.errors.is_some() {
            self.inout_pending = Some(p);
            return Ok(call);
        }
        let held = self.new_local(tuple);
        let mut stmts = vec![Stmt::Let(held, call)];
        stmts.extend(self.give_back(&p, held, tuple, line)?);
        let value = (ty != Type::Unit).then(|| Box::new(self.tuple_item(held, tuple, 0, ty, line)));
        Ok(ir::Expr {
            kind: K::Block(Block { stmts, value }),
            ty,
            line,
        })
    }

    /// The statements that put each `inout` value of `held`, the tuple of
    /// call `p`, into the place its argument names.
    pub(crate) fn give_back(
        &mut self,
        p: &Pending<'a>,
        held: Local,
        tuple: Type,
        line: u32,
    ) -> Lowered<Vec<Stmt>> {
        let first = usize::from(p.ty != Type::Unit);
        let mut stmts = Vec::new();
        for (k, (pos, name)) in p.inout.iter().enumerate() {
            let named = p
                .args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == *name));
            let positional = || p.args.iter().filter(|a| a.name.is_none()).nth(*pos);
            let Some(arg) = named.or_else(positional) else {
                continue;
            };
            let part = self.tuple_item(held, tuple, first + k, p.types[k], line);
            stmts.push(self.assign_path(&arg.value, part)?);
        }
        Ok(stmts)
    }

    /// After `LetResult` put call `p`'s tuple in `held` and its error in
    /// `error`: when it succeeded, the `inout` values go back and its own
    /// value goes into `value`.
    pub(crate) fn give_back_on_success(
        &mut self,
        p: &Pending<'a>,
        held: Local,
        value: Local,
        error: Local,
        line: u32,
    ) -> Lowered<Stmt> {
        let tuple = self.locals[held as usize];
        let mut stmts = self.give_back(p, held, tuple, line)?;
        if p.ty != Type::Unit {
            let own = self.tuple_item(held, tuple, 0, p.ty, line);
            stmts.push(Stmt::Let(value, own));
        }
        let then = Block { stmts, value: None };
        Ok(Stmt::Expr(ir::Expr {
            kind: K::If(
                Box::new(self.succeeded(error, line)),
                then,
                Block::default(),
            ),
            ty: Type::Unit,
            line,
        }))
    }

    /// The outcome of `call` (of kind `kind`, giving `tuple`), a call
    /// that can fail with `inout` parameters (spec 3.9): its values go
    /// back only when it succeeds; when it fails, the places keep theirs.
    pub(crate) fn inout_outcome(
        &mut self,
        p: Pending<'a>,
        kind: K,
        tuple: Type,
        err: Type,
        plain: Option<Type>,
        line: u32,
    ) -> Lowered<crate::lower_errors::Outcome> {
        let (held, error) = (self.new_local(tuple), self.new_local(err));
        let value = self.new_local(p.ty);
        let call = ir::Expr {
            kind,
            ty: tuple,
            line,
        };
        let stmts = vec![
            Stmt::LetResult {
                value: held,
                error,
                call,
            },
            self.give_back_on_success(&p, held, value, error, line)?,
        ];
        let stmt = Stmt::Expr(ir::Expr {
            kind: K::Block(Block { stmts, value: None }),
            ty: Type::Unit,
            line,
        });
        Ok(crate::lower_errors::Outcome {
            stmt,
            value,
            error,
            ok: p.ty,
            err,
            plain,
        })
    }

    /// The declaration of method `name` of the interface type `ty`.
    pub(crate) fn iface_method_decl(&self, ty: Type, name: &str) -> Option<&'a ast::FnDecl> {
        let span = self.c.resolution.defs[self.shapes.def_of(ty)?].span;
        self.c.file.items.iter().find_map(|item| match &item.kind {
            ast::ItemKind::Interface(i) if Some(i.name.span) == span => {
                i.methods.iter().find_map(|m| match &m.kind {
                    ast::ItemKind::Fn(f) if f.name.name == name => Some(f),
                    _ => None,
                })
            }
            _ => None,
        })
    }

    pub(crate) fn tuple_item(
        &self,
        tuple: Local,
        tuple_ty: Type,
        i: usize,
        ty: Type,
        line: u32,
    ) -> ir::Expr {
        let whole = ir::Expr {
            kind: K::Local(tuple),
            ty: tuple_ty,
            line,
        };
        let at = ir::Expr {
            kind: K::Int(i as i128),
            ty: Type::I64,
            line,
        };
        ir::Expr {
            kind: K::Index(Box::new(whole), Box::new(at)),
            ty,
            line,
        }
    }
}
