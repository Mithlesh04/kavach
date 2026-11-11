//! Reading a `Json` with a constant key (decision 0082): `j["key"]` and
//! `j["key"].int() ?? fallback` (or `.float()`) call the runtime with the
//! key's bytes, so no text object is made for the key, and the number is
//! read in place (`runtime/native/src/rt_json_read.rs`).

use crate::func::FnEmit;
use kavach_kir::{Expr, ExprKind, RtOp};

impl FnEmit<'_> {
    /// `e` when it is one of the reads above; `None` for anything else.
    pub(crate) fn json_read(&mut self, e: &Expr) -> Option<Option<String>> {
        let ExprKind::Rt(op, args) = &e.kind else {
            return None;
        };
        match op {
            RtOp::JsonIntOr | RtOp::JsonFloatOr => {
                let ExprKind::Rt(RtOp::JsonGet, get) = &args[0].kind else {
                    return None;
                };
                let ExprKind::Str(key) = &get[1].kind else {
                    return None;
                };
                let float = i64::from(*op == RtOp::JsonFloatOr);
                Some(self.number_or(&get[0], key, &args[1], float, e))
            }
            RtOp::JsonGet => {
                let ExprKind::Str(key) = &args[1].kind else {
                    return None;
                };
                Some(self.get_text(&args[0], key))
            }
            _ => None,
        }
    }

    fn number_or(
        &mut self,
        j: &Expr,
        key: &str,
        fallback: &Expr,
        float: i64,
        e: &Expr,
    ) -> Option<String> {
        let (j, owned) = self.borrow(j, true)?;
        let (ptr, len) = self.m.text(key);
        let fallback = self.slot_of(fallback);
        let args = [
            ("i64", j.clone()),
            ("i64", ptr),
            ("i64", len.to_string()),
            ("i64", fallback),
            ("i64", float.to_string()),
        ];
        let slot = self.call_rt("kv_json_get_number_or", "i64", &args, false);
        if owned {
            self.release(&j);
        }
        Some(self.unpack(&slot, e.ty))
    }

    fn get_text(&mut self, j: &Expr, key: &str) -> Option<String> {
        let (j, owned) = self.borrow(j, true)?;
        let (ptr, len) = self.m.text(key);
        let args = [("i64", j.clone()), ("i64", ptr), ("i64", len.to_string())];
        let slot = self.call_rt("kv_json_get_text", "i64", &args, false);
        if owned {
            self.release(&j);
        }
        Some(slot)
    }
}
