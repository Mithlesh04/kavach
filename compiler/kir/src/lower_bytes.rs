//! `b.read<T>(at, big_endian)` and `Bytes.of<T>(n, big_endian)` (spec
//! 2.13), each an op of the bytes entry (`runtime/native/src/rt_bytes.rs`).
//! `read`'s result type names `T`; `of` passes the size of `T`.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `b.read<T>(at, big_endian: false)`: the bytes, `at` and the flag.
    pub(crate) fn bytes_read(
        &mut self,
        receiver: ir::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let [at, big] = self.number_args(args, "at", line)?;
        Ok(self.at(K::Rt(RtOp::BytesRead, vec![receiver, at, big]), ty, line))
    }

    /// `Bytes.of<T>(n, big_endian: false)`: `n`, the flag and the size.
    pub(crate) fn bytes_of(
        &mut self,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let [n, big] = self.number_args(args, "n", line)?;
        let size = match n.ty {
            Type::I8 | Type::U8 => 1,
            Type::I16 | Type::U16 => 2,
            Type::I32 | Type::U32 | Type::F32 => 4,
            _ => 8,
        };
        let size = self.at(K::Int(size), Type::I64, line);
        Ok(self.at(K::Rt(RtOp::BytesOfNumber, vec![n, big, size]), ty, line))
    }

    /// The two arguments, `first` and `big_endian` (`false` when not
    /// given), by name or in order.
    fn number_args(
        &mut self,
        args: &'a [ast::Arg],
        first: &str,
        line: u32,
    ) -> Lowered<[ir::Expr; 2]> {
        let params = [first, "big_endian"];
        let mut given: [Option<&'a ast::Expr>; 2] = [None, None];
        for (i, a) in args.iter().enumerate() {
            let at = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(slot) = at.and_then(|p| given.get_mut(p)) {
                *slot = Some(&a.value);
            }
        }
        let value = match given[0] {
            Some(e) => self.expr(e)?,
            None => self.at(K::Int(0), Type::I64, line),
        };
        let big = match given[1] {
            Some(e) => self.expr(e)?,
            None => self.at(K::Bool(false), Type::Bool, line),
        };
        Ok([value, big])
    }
}
