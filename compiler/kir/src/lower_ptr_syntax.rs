//! C's pointer syntax on `Ptr<T>` in native code (spec 6.4): `*p` and
//! `p[i]` read, `*p = v` and `p[i] = v` write, `p + n` and `p - n` move
//! the address. The same loads, stores and address arithmetic as
//! `p.read(i)` and `p.write(i, v)` (`lower_ptr.rs`): no checks, as in C.

use crate::ir::{self, ExprKind as K, FastOp, Type};
use crate::lower::{Lowered, Lowerer};
use crate::lower_ptr::Fields;
use crate::rt_op::RtOp;
use kavach_ast as ast;

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

impl<'a> Lowerer<'a> {
    /// Whether `e` is a `Ptr`.
    pub(crate) fn is_ptr(&self, e: &ast::Expr) -> bool {
        matches!(
            self.type_at(e.span),
            Some(kavach_checker::Ty::Builtin("Ptr", _))
        )
    }

    /// The address of value `index` (0 when `None`) of pointer `base`,
    /// with the type it points to.
    fn address(
        &mut self,
        base: &'a ast::Expr,
        index: Option<&'a ast::Expr>,
        line: u32,
    ) -> Lowered<(ir::Expr, Type, Fields)> {
        let (t, size, fields) = self.pointee(self.type_at(base.span), base.span)?;
        let p = self.expr(base)?;
        let Some(index) = index else {
            return Ok((p, t, fields));
        };
        let i = self.expr(index)?;
        let size = at(K::Int(size), Type::I64, line);
        let offset = at(
            K::Fast(FastOp::Mul, Box::new(i), Box::new(size)),
            Type::I64,
            line,
        );
        let addr = at(
            K::Fast(FastOp::Add, Box::new(p), Box::new(offset)),
            Type::I64,
            line,
        );
        Ok((addr, t, fields))
    }

    /// `*p` (no `index`) or `p[i]`.
    pub(crate) fn ptr_load(
        &mut self,
        base: &'a ast::Expr,
        index: Option<&'a ast::Expr>,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let (addr, t, fields) = self.address(base, index, line)?;
        match fields {
            Some(fields) => self.record_access(t, &fields, addr, None, line),
            None => Ok(at(K::Rt(RtOp::PtrRead, vec![addr]), t, line)),
        }
    }

    /// `*p = value` (no `index`) or `p[i] = value`.
    pub(crate) fn ptr_store(
        &mut self,
        base: &'a ast::Expr,
        index: Option<&'a ast::Expr>,
        value: ir::Expr,
    ) -> Lowered<ir::Expr> {
        let line = value.line;
        let (addr, t, fields) = self.address(base, index, line)?;
        let value = self.coerce(value, t)?;
        match fields {
            Some(fields) => self.record_access(t, &fields, addr, Some(value), line),
            None => Ok(at(
                K::Rt(RtOp::PtrWrite, vec![addr, value]),
                Type::Unit,
                line,
            )),
        }
    }

    /// `p + n` or `p - n` (`back`): the address `n` values on.
    pub(crate) fn ptr_moved(
        &mut self,
        base: &'a ast::Expr,
        n: &'a ast::Expr,
        back: bool,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let (_, size, _) = self.pointee(self.type_at(base.span), base.span)?;
        let p = self.expr(base)?;
        let n = self.expr(n)?;
        let size = at(K::Int(size), Type::I64, line);
        let offset = at(
            K::Fast(FastOp::Mul, Box::new(n), Box::new(size)),
            Type::I64,
            line,
        );
        let op = if back { FastOp::Sub } else { FastOp::Add };
        Ok(at(
            K::Fast(op, Box::new(p), Box::new(offset)),
            Type::I64,
            line,
        ))
    }
}
