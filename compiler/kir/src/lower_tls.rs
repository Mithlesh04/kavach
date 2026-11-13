//! `net.connect_tls` and `net.listen_tls` (spec 14.10) in native code. A
//! runtime op takes three operands, so the call becomes: the address, the
//! protocols, and the other texts in a list (`roots`, `cert`, `key` for a
//! client; `cert`, `key`, `clients` for a listener), each in its place
//! however the call names them, and its default when left out.

use crate::ir::{self, ExprKind as K};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn tls_call(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: ir::Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let (op, params): (RtOp, [&str; 5]) = if name == "listen_tls" {
            (
                RtOp::NetListenTls,
                ["address", "cert", "key", "protocols", "clients"],
            )
        } else {
            (
                RtOp::NetConnectTls,
                ["address", "protocols", "roots", "cert", "key"],
            )
        };
        // Each argument in its parameter's place: positional, then named.
        let mut given: [Option<&'a ast::Expr>; 5] = [None; 5];
        for (i, a) in args.iter().enumerate() {
            let at = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(at) = at.filter(|at| *at < 5) {
                given[at] = Some(&a.value);
            }
        }
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let text = self.str_type();
        let texts = self.list_type(text);
        let mut values = Vec::new();
        for (p, g) in params.iter().zip(given) {
            values.push(match (g, *p) {
                (Some(e), _) => self.expr(e)?,
                (None, "protocols") => at(K::List(Vec::new()), texts),
                (None, "address") => return Err(self.unsupported(format!("`net.{name}()`"), span)),
                (None, _) => at(K::Str(String::new()), text),
            });
        }
        let address = values.remove(0);
        let protocols = values.remove(if name == "listen_tls" { 2 } else { 0 });
        let rest = at(K::List(values), texts);
        Ok(at(K::Rt(op, vec![address, protocols, rest]), ty))
    }

    /// `conn.start_tls(host, …)` or `conn.accept_tls(cert, key, …)` (spec
    /// 14.10): the connection, the protocols, and the other texts in a
    /// list, each in its place however the call names them.
    pub(crate) fn start_tls_call(
        &mut self,
        base: &'a ast::Expr,
        name: &str,
        args: &'a [ast::Arg],
        ty: ir::Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let (op, params): (RtOp, &[&str]) = if name == "start_tls" {
            (
                RtOp::NetStartTls,
                &["host", "protocols", "roots", "cert", "key"],
            )
        } else {
            (RtOp::NetAcceptTls, &["cert", "key", "protocols", "clients"])
        };
        let mut given: Vec<Option<&'a ast::Expr>> = vec![None; params.len()];
        for (i, a) in args.iter().enumerate() {
            let at = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(slot) = at.and_then(|at| given.get_mut(at)) {
                *slot = Some(&a.value);
            }
        }
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let text = self.str_type();
        let texts = self.list_type(text);
        let conn = self.expr(base)?;
        let (mut protocols, mut rest) = (at(K::List(Vec::new()), texts), Vec::new());
        for (p, g) in params.iter().zip(given) {
            let value = match g {
                Some(e) => self.expr(e)?,
                None if *p == "protocols" => at(K::List(Vec::new()), texts),
                None => at(K::Str(String::new()), text),
            };
            if *p == "protocols" {
                protocols = value;
            } else {
                rest.push(value);
            }
        }
        let rest = at(K::List(rest), texts);
        Ok(at(K::Rt(op, vec![conn, protocols, rest]), ty))
    }
}
