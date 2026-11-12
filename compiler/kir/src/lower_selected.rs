//! Standard library functions carried out by one op each family, the
//! function chosen by its number in operand `c`: `std.crypto` (spec
//! 14.19, `RtOp::Crypto` and `CryptoFails`) and a WebSocket handshake's
//! pieces (decision 0092, `RtOp::NetWebSocket`).

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

/// The `std.crypto` functions that can fail.
const FAILING: [&str; 6] = [
    "ed25519_public",
    "ed25519_sign",
    "p256_public",
    "p256_sign",
    "encrypt",
    "decrypt",
];

/// The WebSocket pieces, by number.
const WEBSOCKET: [&str; 3] = ["websocket_accept", "websocket_key", "websocket_mask"];

impl<'a> Lowerer<'a> {
    /// `crypto.rsa_verify(n, e, data, signature, pss: false)`: the four
    /// `Bytes` in one list (operand `a`), `pss` in `b`.
    pub(crate) fn rsa_verify(
        &mut self,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        const PARAMS: [&str; 5] = ["n", "e", "data", "signature", "pss"];
        let mut given: [Option<&'a ast::Expr>; 5] = [None; 5];
        for (i, a) in args.iter().enumerate() {
            let at = match &a.name {
                Some(n) => PARAMS.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(slot) = at.and_then(|p| given.get_mut(p)) {
                *slot = Some(&a.value);
            }
        }
        let mut parts = Vec::new();
        for e in &given[..4] {
            let e = e.ok_or_else(|| self.unsupported("`crypto.rsa_verify()`", span))?;
            parts.push(self.expr(e)?);
        }
        let item = parts[0].ty;
        let list = self.list_type(item);
        let pss = match given[4] {
            Some(e) => self.expr(e)?,
            None => self.at(K::Int(0), Type::Bool, line),
        };
        let parts = self.at(K::List(parts), list, line);
        Ok(self.at(K::Rt(RtOp::CryptoRsaVerify, vec![parts, pss]), ty, line))
    }

    /// `module.name(args)`: its arguments in operands `a` and `b`, its
    /// number in `c`.
    pub(crate) fn selected_call(
        &mut self,
        module: &str,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let (op, number) = if module == "net" {
            let n = WEBSOCKET.iter().position(|w| *w == name).unwrap_or(0);
            (RtOp::NetWebSocket, n)
        } else {
            let n = crate::CRYPTO.iter().position(|c| *c == name).unwrap_or(0);
            let op = if FAILING.contains(&name) {
                RtOp::CryptoFails
            } else {
                RtOp::Crypto
            };
            (op, n)
        };
        let mut values = Vec::new();
        for a in args {
            values.push(self.expr(&a.value)?);
        }
        while values.len() < 2 {
            values.push(self.at(K::Int(0), Type::I64, line));
        }
        values.push(self.at(K::Int(number as i128), Type::I64, line));
        Ok(self.at(K::Rt(op, values), ty, line))
    }
}
