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
