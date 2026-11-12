//! `std.base64` (spec 14.20): `base64.encode(data, url, pad)` and
//! `base64.decode(text, url)`, each an op of the runtime's own entry
//! (`runtime/native/src/rt_base64.rs`), its flags given in place.

use crate::ir::{self, ExprKind as K, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    pub(crate) fn base64_call(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let (op, params, defaults): (RtOp, &[&str], &[i128]) = match name {
            "encode" => (RtOp::Base64Encode, &["data", "url", "pad"], &[0, 0, 1]),
            "decode" => (RtOp::Base64Decode, &["text", "url"], &[0, 0]),
            _ => return Err(self.unsupported(format!("`base64.{name}()`"), span)),
        };
        let mut given: Vec<Option<&'a ast::Expr>> = vec![None; params.len()];
        for (i, a) in args.iter().enumerate() {
            let at = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(slot) = at.and_then(|p| given.get_mut(p)) {
                *slot = Some(&a.value);
            }
        }
        let mut values = Vec::new();
        for (e, default) in given.into_iter().zip(defaults) {
            values.push(match e {
                Some(e) => self.expr(e)?,
                None => self.at(K::Int(*default), Type::Bool, line),
            });
        }
        Ok(self.at(K::Rt(op, values), ty, line))
    }
}
