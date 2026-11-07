//! Code for errors as values (spec 5) and for storing into locals.
//!
//! A function that can fail returns its error after its value: a pointer
//! that is 0 on success. `fail` releases the locals and returns that
//! pointer with a placeholder value.

use crate::codegen::FnGen;
use cranelift_codegen::ir::{Value, types};
use kavach_kir::{Expr, ExprKind, Local, RtOp};

impl FnGen<'_, '_> {
    /// `fail error`: returns from the function with `error`.
    pub(crate) fn fail(&mut self, error: &Expr) {
        let Some(err) = self.expr(error) else { return };
        let ret = self.ret;
        let value = self.zero(ret);
        self.leave_and_return(value, Some(err));
    }

    /// Calls something that can fail, keeping its value and error.
    pub(crate) fn let_result(&mut self, value: Local, error: Local, call: &Expr) {
        let (v, err) = match &call.kind {
            ExprKind::Call(f, args) if self.fails[*f as usize] => {
                let Some((values, made)) = self.call_args(*f, args) else {
                    return;
                };
                let id = self.funcs[*f as usize];
                let results = self.call(id, &values);
                if let Some(c) = made {
                    self.release(c);
                }
                let err = *results.last().expect("an error result");
                (
                    results
                        .get(..results.len() - 1)
                        .and_then(|r| r.first().copied()),
                    err,
                )
            }
            // Through a function value: in a `LetResult`, a failing one (an
            // interface's failing method, `kir/src/lower_iface.rs`).
            ExprKind::CallValue(callee, args) => {
                let Some(results) = self.call_value(callee, args, call.ty, true, call.line) else {
                    return;
                };
                let err = *results.last().expect("an error result");
                (
                    results
                        .get(..results.len() - 1)
                        .and_then(|r| r.first().copied()),
                    err,
                )
            }
            ExprKind::Parse(text) => {
                let Some(t) = self.expr(text) else { return };
                let (code, str_code) = (self.code(call.ty), self.code(text.ty));
                let slot = self.call_rt("kv_parse", &[t, code, str_code])[0];
                let err = self.call_rt("kv_take_error", &[])[0];
                (Some(self.unpack(slot, call.ty)), err)
            }
            ExprKind::Rt(op, args) if op.can_fail() => {
                let a = self.slot_of(&args[0]);
                let v = self.rt_call(*op, a, &args[1..], call.ty, call.line);
                let err = self.call_rt("kv_take_error", &[])[0];
                ((call.ty != kavach_kir::Type::Unit).then_some(v), err)
            }
            // A call that cannot fail after all: it always succeeds.
            _ => (self.expr(call), self.int(types::I64, 0)),
        };
        if let Some(v) = v {
            self.set_local(value, v);
        }
        self.set_local(error, err);
    }

    /// `local = value`. When `value` changes the same local's list or
    /// record (`items.push(x)`, `p.x = 1`), the local's reference moves
    /// into the change, so an unshared value is changed in place instead
    /// of copied.
    pub(crate) fn assign(&mut self, local: Local, value: &Expr) {
        let var = self.vars[local as usize];
        match &value.kind {
            ExprKind::Rt(RtOp::ListPush, args)
                if self.flat_width(value.ty) > 0 && is_local(&args[0], local) =>
            {
                // A record built right here goes into the list field by field.
                let item = self.flat_item(&args[1]);
                let current = self.b.use_var(var);
                let changed = self.flat_push(current, item, value.ty, value.line);
                self.b.def_var(var, changed);
            }
            ExprKind::Rt(op, args) if op.changes_receiver() && is_local(&args[0], local) => {
                // The other operands first: they may read the local.
                let mut slots: Vec<Value> = args[1..].iter().map(|x| self.slot_of(x)).collect();
                while slots.len() < 2 {
                    slots.push(self.int(types::I64, 0));
                }
                let current = self.b.use_var(var);
                let list = self.is_list(value.ty);
                let changed = match op {
                    RtOp::ListPush if self.text_list(value.ty) => {
                        self.text_push(current, slots[0], value.ty, value.line)
                    }
                    // The runtime replaces an item of a list of text.
                    RtOp::ListSet if self.text_list(value.ty) => {
                        self.rt_slots(*op, current, slots[0], slots[1], value.ty, value.line)
                    }
                    RtOp::ListPush if list => {
                        self.list_push(current, slots[0], value.ty, value.line)
                    }
                    RtOp::ListSet if self.flat_width(value.ty) > 0 => {
                        self.flat_set(current, slots[0], slots[1], value.ty, value.line)
                    }
                    RtOp::ListSet if list => {
                        self.list_set(current, slots[0], slots[1], value.ty, value.line)
                    }
                    RtOp::MapInsert if self.is_map(value.ty) => {
                        self.call_rt("kv_map_insert", &[current, slots[0], slots[1]])[0]
                    }
                    _ => self.rt_slots(*op, current, slots[0], slots[1], value.ty, value.line),
                };
                self.b.def_var(var, changed);
            }
            ExprKind::SetField(base, i, field) if is_local(base, local) => {
                let slot = self.slot_of(field);
                let current = self.b.use_var(var);
                let changed = if self.has_fixed_fields(base.ty) {
                    self.set_field(current, *i, slot, field.ty)
                } else {
                    let index = self.int(types::I64, *i as i64);
                    self.call_rt("kv_set", &[current, index, slot])[0]
                };
                self.b.def_var(var, changed);
            }
            _ => {
                if let Some(v) = self.expr(value) {
                    self.set_local(local, v);
                }
            }
        }
    }
}

fn is_local(e: &Expr, local: Local) -> bool {
    matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if l == local) && e.ty.is_ref()
}
