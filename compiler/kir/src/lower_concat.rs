//! `xs + ys` and `xs += ys` on lists (spec 2.2), as pushes onto a list,
//! so each backend's kinds of list (flat, text, packed) need nothing new;
//! and `xs.pop()` (spec 14.2), as its last item and a shorter list.

use crate::ir::{self, Block, ExprKind as K, Local, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// Whether `ty` is a list type.
    pub(crate) fn is_list_type(&self, ty: Type) -> bool {
        matches!(self.shapes.shape(ty), Some(Shape::List(_)))
    }

    /// `xs + ys`: a copy of `xs`, with each item of `ys` pushed onto it.
    /// The copy is made by the first push, only when `xs` is still held.
    pub(crate) fn list_concat(&mut self, l: ir::Expr, r: ir::Expr, line: u32) -> Lowered<ir::Expr> {
        let ty = l.ty;
        let r = self.coerce(r, ty)?;
        let acc = self.new_local(ty);
        let mut stmts = vec![Stmt::Let(acc, l)];
        stmts.extend(self.push_all(acc, r, line));
        Ok(ir::Expr {
            kind: K::Block(Block {
                stmts,
                value: Some(Box::new(self.local_expr(acc, line))),
            }),
            ty,
            line,
        })
    }

    /// `xs += ys` with `xs` a variable: each item of `ys` pushed onto it,
    /// in place when nothing else holds it (spec 6.1).
    pub(crate) fn list_extend(
        &mut self,
        target: Local,
        r: ir::Expr,
        line: u32,
    ) -> Lowered<Vec<Stmt>> {
        let r = self.coerce(r, self.locals[target as usize])?;
        Ok(self.push_all(target, r, line))
    }

    /// `xs.pop()`: the last item (`none` for an empty list), and `xs` cut
    /// to the items before it, in place when nothing else holds it. `None`
    /// when `base` is not a list.
    pub(crate) fn list_pop(
        &mut self,
        base: &'a ast::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let typed = self.c.expr_types.get(&(base.span.start, base.span.end));
        if !matches!(typed, Some(kavach_checker::Ty::List(_))) {
            return Ok(None);
        }
        let list = self.expr(base)?;
        let list_ty = list.ty;
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (len, item) = (self.new_local(Type::I64), self.new_local(ty));
        let last = at(K::Rt(RtOp::ListLast, vec![self.expr(base)?]), ty);
        let one = at(K::Int(1), Type::I64);
        let keep = at(
            K::Binary(
                ir::BinOp::Sub,
                Box::new(self.local_expr(len, line)),
                Box::new(one),
            ),
            Type::I64,
        );
        let cut = at(
            K::Rt(RtOp::ListTruncate, vec![self.expr(base)?, keep]),
            list_ty,
        );
        let stmts = vec![
            Stmt::Let(len, at(K::Rt(RtOp::ListLen, vec![list]), Type::I64)),
            Stmt::Let(item, last),
            self.assign_path(base, cut)?,
        ];
        Ok(Some(at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(self.local_expr(item, line))),
            }),
            ty,
        )))
    }

    /// `for item in r { acc = acc.push(item) }`.
    fn push_all(&mut self, acc: Local, r: ir::Expr, line: u32) -> Vec<Stmt> {
        let list_ty = self.locals[acc as usize];
        let item_ty = match self.shapes.shape(list_ty) {
            Some(Shape::List(t)) => *t,
            _ => unreachable!("a list"),
        };
        let src = self.new_local(r.ty);
        let (item, index) = (self.new_local(item_ty), self.new_local(Type::I64));
        let pushed = ir::Expr {
            kind: K::Rt(
                RtOp::ListPush,
                vec![self.local_expr(acc, line), self.local_expr(item, line)],
            ),
            ty: list_ty,
            line,
        };
        let set = ir::Expr {
            kind: K::Assign(acc, Box::new(pushed)),
            ty: Type::Unit,
            line,
        };
        vec![
            Stmt::Let(src, r),
            Stmt::ForEach {
                var: item,
                index,
                list: src,
                body: Block {
                    stmts: vec![Stmt::Expr(set)],
                    value: None,
                },
            },
        ]
    }
}
