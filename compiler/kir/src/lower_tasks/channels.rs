//! `for v in receive` (spec 9.4): a loop that takes from the channel
//! until it gives `none`.

use super::{at, local};
use crate::ir::{self, Block, ExprKind as K, Shape, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// `for v in receive` (spec 9.4): `while true { let o = take(); if o
    /// is none { break }; v = o.unwrap(); … }`.
    pub(crate) fn receive_loop(
        &mut self,
        pattern: &'a ast::Pattern,
        iter: &'a ast::Expr,
        body: &'a ast::Block,
        out: &mut Vec<Stmt>,
    ) -> Lowered<()> {
        let channel = self.expr(iter)?;
        let line = channel.line;
        // A channel's values, or a file's lines or pieces (spec 14.3).
        let (item_ty, next) = match self.shapes.shape(channel.ty).cloned() {
            Some(Shape::Channel(t)) => (t, RtOp::ChanTake),
            Some(Shape::Reader(t)) => (t, RtOp::ReaderNext),
            _ => return Err(self.unsupported("`for` over this", iter.span)),
        };
        let chan_ty = channel.ty;
        let chan = self.new_local(chan_ty);
        out.push(Stmt::Let(chan, channel));
        let opt_ty = self.shapes_intern(Shape::Optional(item_ty));
        let (taken, var) = (self.new_local(opt_ty), self.new_local(item_ty));
        // A reader whose header has `try` or `catch` sends a failed read
        // there (spec 14.3).
        let handled = match &iter.kind {
            ast::ExprKind::Try { expr, .. } | ast::ExprKind::Catch { expr, .. }
                if next == RtOp::ReaderNext =>
            {
                Some(expr)
            }
            _ => None,
        };
        let next = if handled.is_some() {
            RtOp::ReaderTryNext
        } else {
            next
        };
        let take = K::Rt(next, vec![local(chan, chan_ty, line)]);
        let mut first = Vec::new();
        match handled {
            Some(call) => {
                let err = self.read_error_type(call)?;
                let error = self.new_local(err);
                first.push(Stmt::LetResult {
                    value: taken,
                    error,
                    call: at(take, opt_ty, line),
                });
                let on_error = self.header_failed(iter, error, err, line)?;
                first.push(Stmt::Expr(at(
                    K::If(
                        Box::new(self.failed(error, line)),
                        on_error,
                        Block::default(),
                    ),
                    Type::Unit,
                    line,
                )));
            }
            None => first.push(Stmt::Let(taken, at(take, opt_ty, line))),
        }
        let present = at(
            K::IsSome(Box::new(local(taken, opt_ty, line))),
            Type::Bool,
            line,
        );
        let closed = K::If(
            Box::new(at(
                K::Unary(ir::UnOp::Not, Box::new(present)),
                Type::Bool,
                line,
            )),
            Block {
                stmts: vec![Stmt::Break],
                value: None,
            },
            Block::default(),
        );
        first.push(Stmt::Expr(at(closed, Type::Unit, line)));
        first.push(Stmt::Let(
            var,
            at(
                K::Unwrap(Box::new(local(taken, opt_ty, line))),
                item_ty,
                line,
            ),
        ));
        let mut b = Block {
            stmts: first,
            value: None,
        };
        match &pattern.kind {
            ast::PatternKind::Binding(id) => self.bind(id, var),
            ast::PatternKind::Wildcard => {}
            _ => self.destructure(pattern, var, item_ty, &mut b.stmts)?,
        }
        self.block_into(body, false, &mut b)?;
        out.push(Stmt::While {
            cond: at(K::Bool(true), Type::Bool, line),
            body: b,
        });
        Ok(())
    }

    /// `x.timeout(d)` (spec 9.6), where `x` is `task.wait()`,
    /// `receive.take()` or `send.put(v)`: that wait, for at most `d`, as
    /// the interpreter runs it (`d` first). `None` for any other `x`.
    pub(crate) fn timeout(
        &mut self,
        x: &'a ast::Expr,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let ast::ExprKind::Call {
            callee,
            args: inner,
            ..
        } = &x.kind
        else {
            return Ok(None);
        };
        let ast::ExprKind::Field { base, name, .. } = &callee.kind else {
            return Ok(None);
        };
        let op = match name.name.as_str() {
            "wait" => RtOp::TaskWaitFor,
            "take" => RtOp::ChanTakeFor,
            "put" => RtOp::ChanPutFor,
            "accept" | "read" | "write" | "send" | "receive" | "read_stream" | "write_stream"
            | "connect" | "connect_tls" | "connect_quic" => RtOp::NetDeadline,
            _ => return Ok(None),
        };
        let Some(d) = args.first() else {
            return Ok(None);
        };
        let d = self.expr(&d.value)?;
        let ms = at(K::GetField(Box::new(d), 0), Type::I64, line);
        if op == RtOp::NetDeadline {
            return self.net_timeout(x, ms, ty, line);
        }
        let target = self.expr(base)?;
        let mut operands = vec![target];
        if op == RtOp::ChanPutFor {
            let Some(Shape::Channel(item)) = self.shapes.shape(operands[0].ty).cloned() else {
                return Ok(None);
            };
            let Some(v) = inner.first() else {
                return Ok(None);
            };
            let v = self.expr(&v.value)?;
            operands.push(self.coerce(v, item)?);
        }
        operands.push(ms);
        Ok(Some(at(K::Rt(op, operands), ty, line)))
    }

    /// `x.timeout(d)` for a network wait `x` (spec 14.10): `{ let t = ms;
    /// let a = …; NetDeadline(t); op(a, …) }`, so the deadline is set
    /// right before the op, after anything its operands do.
    fn net_timeout(
        &mut self,
        x: &'a ast::Expr,
        ms: ir::Expr,
        ty: Type,
        line: u32,
    ) -> Lowered<Option<ir::Expr>> {
        let call = self.expr(x)?;
        let K::Rt(op, operands) = call.kind else {
            return Ok(None);
        };
        let t = self.new_local(Type::I64);
        let mut stmts = vec![Stmt::Let(t, ms)];
        let mut values = Vec::new();
        for v in operands {
            let (l, vt) = (self.new_local(v.ty), v.ty);
            stmts.push(Stmt::Let(l, v));
            values.push(local(l, vt, line));
        }
        let deadline = K::Rt(RtOp::NetDeadline, vec![local(t, Type::I64, line)]);
        stmts.push(Stmt::Expr(at(deadline, Type::Unit, line)));
        let block = Block {
            stmts,
            value: Some(Box::new(at(K::Rt(op, values), ty, line))),
        };
        Ok(Some(at(K::Block(block), ty, line)))
    }
}
