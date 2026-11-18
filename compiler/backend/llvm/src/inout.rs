//! `RtOp::InoutStash` and `RtOp::InoutTake` (`kavach_kir`'s
//! `inout_stash.rs`): a function with `inout` parameters leaves their
//! values in its thread's `@kv_tls_inout` slots, and the caller takes them
//! back. Here that is a store and a load, with no call into the runtime
//! (Sakshi's S11).

use crate::func::FnEmit;
use crate::module::llvm_type;
use kavach_kir::{Expr, ExprKind, RtOp};

impl FnEmit<'_> {
    /// `e` when it is one of the two ops; `None` for anything else.
    pub(crate) fn inout_slot(&mut self, e: &Expr) -> Option<Option<String>> {
        let ExprKind::Rt(op @ (RtOp::InoutStash | RtOp::InoutTake), args) = &e.kind else {
            return None;
        };
        let ExprKind::Int(slot) = args[0].kind else {
            return None;
        };
        let at = self.val(format!(
            "getelementptr inbounds [8 x i64], ptr @kv_tls_inout, i64 0, i64 {}",
            slot % 8
        ));
        if *op == RtOp::InoutStash {
            let ty = args[1].ty;
            let v = self.expr(&args[1])?;
            let wide = self.widen(&v, ty);
            self.ins(format!("store i64 {wide}, ptr {at}"));
            return Some(None);
        }
        let v = self.val(format!("load i64, ptr {at}"));
        Some(Some(match llvm_type(e.ty) {
            "i64" => v,
            t => self.val(format!("trunc i64 {v} to {t}")),
        }))
    }
}
