//! `list.map(f)` that may run on the GPU (spec 15.2). The runtime's
//! `GpuMap` gives the GPU's list, or `none` when the CPU should do it
//! (a short list, no GPU, `KAVACH_GPU=off`); then the same `map` runs
//! here. The kernel's WGSL is attached after lowering by the `wgsl`
//! backend, from the closure's function; until then its text is empty,
//! which the runtime reads as "use the CPU".

use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn gpu_map(
        &mut self,
        list: ir::Expr,
        f: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        self.gpu_list_op(RtOp::GpuMap, "map", list, f, ty, span)
    }

    /// `list.map(f)` or `list.filter(f)` (`method`) that may run on the
    /// GPU as `op`: the GPU's list, or the CPU's.
    pub(crate) fn gpu_list_op(
        &mut self,
        op: RtOp,
        method: &str,
        list: ir::Expr,
        f: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let closure = self.expr(f)?;
        if !matches!(closure.kind, K::Closure { .. }) {
            return self.list_loop(method, list, f, ty, span);
        }
        let held = self.new_local(list.ty);
        let maybe = self.shapes_intern(Shape::Optional(ty));
        let got = self.new_local(maybe);
        let text = at(K::Str(String::new()), self.str_type());
        let gpu = at(
            K::Rt(op, vec![self.local_expr(held, line), closure, text]),
            maybe,
        );
        let cpu = self.list_loop(method, self.local_expr(held, line), f, ty, span)?;
        // `KAVACH_GPU=check` compares it with the GPU's list (spec 15.2).
        let cpu = at(K::Rt(RtOp::GpuCheck, vec![cpu]), ty);
        let on_gpu = at(K::IsSome(Box::new(self.local_expr(got, line))), Type::Bool);
        let taken = at(K::Unwrap(Box::new(self.local_expr(got, line))), ty);
        let choose = at(
            K::If(
                Box::new(on_gpu),
                Block {
                    stmts: Vec::new(),
                    value: Some(Box::new(taken)),
                },
                Block {
                    stmts: Vec::new(),
                    value: Some(Box::new(cpu)),
                },
            ),
            ty,
        );
        Ok(at(
            K::Block(Block {
                stmts: vec![Stmt::Let(held, list), Stmt::Let(got, gpu)],
                value: Some(Box::new(choose)),
            }),
            ty,
        ))
    }

    /// `list.sum(f)`, `list.min()` or `list.max()` of whole numbers, which
    /// may run on the GPU (spec 15.2): the GPU's value when `GpuReduced`
    /// says it gave one, or else the CPU's, worked out in order. The CPU
    /// replays a sum whose partial sums overflowed on the GPU, so its
    /// panic names the right item.
    pub(crate) fn gpu_reduce(
        &mut self,
        name: &str,
        list: ir::Expr,
        f: Option<&'a ast::Expr>,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let held = self.new_local(list.ty);
        let (op, closure, value_ty) = match (name, f) {
            ("sum", Some(f)) => {
                let closure = self.expr(f)?;
                if !matches!(closure.kind, K::Closure { .. }) {
                    return self.list_loop("sum", list, f, ty, span);
                }
                (RtOp::GpuSum, closure, ty)
            }
            (_, None) => {
                let item = match self.shapes.shape(list.ty) {
                    Some(Shape::List(t)) => *t,
                    _ => return Err(self.unsupported(format!("`{name}` of a list"), span)),
                };
                let op = if name == "min" {
                    RtOp::GpuMin
                } else {
                    RtOp::GpuMax
                };
                (op, at(K::Int(0), Type::I64), item)
            }
            _ => return Err(self.unsupported(format!("`{name}` on the GPU"), span)),
        };
        let value = self.new_local(value_ty);
        let text = at(K::Str(String::new()), self.str_type());
        let gpu = at(
            K::Rt(op, vec![self.local_expr(held, line), closure, text]),
            value_ty,
        );
        let cpu = match f {
            Some(f) => self.list_loop("sum", self.local_expr(held, line), f, ty, span)?,
            None => {
                let op = if name == "min" {
                    RtOp::ListMin
                } else {
                    RtOp::ListMax
                };
                at(K::Rt(op, vec![self.local_expr(held, line)]), ty)
            }
        };
        let got = self.local_expr(value, line);
        let got = match f {
            Some(_) => got,
            None => at(K::Some(Box::new(got)), ty),
        };
        let value_of = |e: ir::Expr| Block {
            stmts: Vec::new(),
            value: Some(Box::new(e)),
        };
        let reduced = at(K::Rt(RtOp::GpuReduced, Vec::new()), Type::Bool);
        let choose = at(K::If(Box::new(reduced), value_of(got), value_of(cpu)), ty);
        Ok(at(
            K::Block(Block {
                stmts: vec![Stmt::Let(held, list), Stmt::Let(value, gpu)],
                value: Some(Box::new(choose)),
            }),
            ty,
        ))
    }

    /// `xs.map(f).map(g)`, both on the GPU (spec 15.2): one kernel that
    /// works out `g(f(x))` for each item, so the list between them never
    /// comes back to the CPU. `None` when it isn't such a chain.
    pub(crate) fn fused_gpu_maps(
        &mut self,
        base: &'a ast::Expr,
        name: &ast::Ident,
        args: &'a [ast::Arg],
        ty: Type,
        span: ast::Span,
    ) -> Option<Lowered<ir::Expr>> {
        let gpu = |s: ast::Span| self.c.gpu_maps.contains(&(s.start, s.end));
        let ([g], "map", true, true) = (args, name.name.as_str(), gpu(span), gpu(base.span)) else {
            return None;
        };
        let ast::ExprKind::Call {
            callee,
            args: inner,
            ..
        } = &base.kind
        else {
            return None;
        };
        let (
            ast::ExprKind::Field {
                base: list,
                name: first,
                ..
            },
            [f],
        ) = (&callee.kind, &inner[..])
        else {
            return None;
        };
        (first.name == "map").then(|| self.gpu_map_chain(list, &f.value, &g.value, base, ty, span))
    }

    fn gpu_map_chain(
        &mut self,
        list: &'a ast::Expr,
        f: &'a ast::Expr,
        g: &'a ast::Expr,
        middle: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (first, second) = (self.expr(f)?, self.expr(g)?);
        if !matches!(first.kind, K::Closure { .. }) || !matches!(second.kind, K::Closure { .. }) {
            let middle = self.expr(middle)?;
            return self.gpu_map(middle, g, ty, span);
        }
        let list = self.expr(list)?;
        let middle_ty = self.ty_of(middle)?;
        let held = self.new_local(list.ty);
        let maybe = self.shapes_intern(Shape::Optional(ty));
        let got = self.new_local(maybe);
        // Both closures, as a pair: the kernel calls one after the other.
        let pair_ty = self.shapes_intern(Shape::Tuple(vec![first.ty, second.ty]));
        let pair = at(K::Tuple(vec![first, second]), pair_ty);
        let text = at(K::Str(String::new()), self.str_type());
        let gpu = at(
            K::Rt(RtOp::GpuMap, vec![self.local_expr(held, line), pair, text]),
            maybe,
        );
        // On the CPU, the two `map`s as written.
        let mid = self.list_loop("map", self.local_expr(held, line), f, middle_ty, span)?;
        let cpu = self.list_loop("map", mid, g, ty, span)?;
        let cpu = at(K::Rt(RtOp::GpuCheck, vec![cpu]), ty);
        let on_gpu = at(K::IsSome(Box::new(self.local_expr(got, line))), Type::Bool);
        let taken = at(K::Unwrap(Box::new(self.local_expr(got, line))), ty);
        let value = |e: ir::Expr| Block {
            stmts: Vec::new(),
            value: Some(Box::new(e)),
        };
        let choose = at(K::If(Box::new(on_gpu), value(taken), value(cpu)), ty);
        Ok(at(
            K::Block(Block {
                stmts: vec![Stmt::Let(held, list), Stmt::Let(got, gpu)],
                value: Some(Box::new(choose)),
            }),
            ty,
        ))
    }
}
