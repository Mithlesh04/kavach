//! Calls: of program functions and through function values. A closure's
//! function borrows its closure (`kavach_kir::borrows`): the caller keeps
//! its reference while the call runs, and lets go of a closure it made
//! just for the call afterwards.

use crate::func::{FnEmit, fn_name};
use crate::module::llvm_type;
use kavach_kir::{Expr, ExprKind, Type};
use kavach_runtime_native::heap::FIELDS;

impl FnEmit<'_> {
    /// A closure for a call that borrows it (`kavach_kir::borrows`), and
    /// whether it was made for the call (then it is let go of after): a
    /// local is read in place unless an argument gives it a new value.
    pub(crate) fn callee(&mut self, c: &Expr, args: &[Expr]) -> Option<(String, bool)> {
        let changed = match c.kind {
            ExprKind::Local(l) | ExprKind::Move(l) => args.iter().any(|a| {
                kavach_kir::walk::any_expr(
                    a,
                    &mut |x| matches!(x.kind, ExprKind::Assign(m, _) if m == l),
                )
            }),
            _ => false,
        };
        self.borrow(c, !changed)
    }

    /// A call of program function `f`; `Unit` arguments are left out, as
    /// in its signature.
    /// The result of a function that can fail is its value and its error
    /// (see `module::ret_type`).
    pub(crate) fn call(&mut self, f: usize, args: &[Expr], ty: Type, line: u32) -> Option<String> {
        let mut list = Vec::new();
        // A closure's function borrows its closure: let go of after the
        // call when it was made for it.
        let mut made = None;
        let rest = match args.split_first() {
            Some((c, rest)) if self.m.closures[f] => {
                let (v, owned) = self.callee(c, rest)?;
                list.push(format!("{} {v}", llvm_type(c.ty)));
                made = owned.then_some(v);
                rest
            }
            _ => args,
        };
        for a in rest {
            if let Some(v) = self.expr(a) {
                list.push(format!("{} {v}", llvm_type(a.ty)));
            }
        }
        self.stack_check(line);
        // `notail`: LLVM must not turn recursion into a loop, so running
        // out of stack stays a panic (spec 3.8) instead of a loop forever.
        let ret = crate::module::ret_type(ty, self.m.fails[f]);
        let name = self.m.c_names[f].clone().unwrap_or_else(|| fn_name(f));
        let call = format!("notail call {ret} @{name}({})", list.join(", "));
        let result = if ret == "void" {
            self.ins(call);
            None
        } else {
            Some(self.val(call))
        };
        if let Some(c) = made {
            self.release(&c);
        }
        result
    }

    /// A call through function value `callee` (a closure, which its
    /// function borrows), giving `ty`; when it `fails`, its value and its
    /// error, as `module::ret_type` says.
    pub(crate) fn call_value(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        ty: Type,
        fails: bool,
        line: u32,
    ) -> Option<String> {
        // The closure is borrowed (`kavach_kir::borrows`): let go of after
        // the call when it was made for it.
        let (f, made) = self.callee(callee, args)?;
        let mut list = vec![format!("i64 {f}")];
        for a in args {
            if let Some(v) = self.expr(a) {
                list.push(format!("{} {v}", llvm_type(a.ty)));
            }
        }
        let addr = self.load_item_at(&f, FIELDS);
        let target = self.val(format!("inttoptr i64 {addr} to ptr"));
        self.stack_check(line);
        let ret = crate::module::ret_type(ty, fails);
        let call = format!("notail call {ret} {target}({})", list.join(", "));
        let result = if ret == "void" {
            self.ins(call);
            None
        } else {
            Some(self.val(call))
        };
        if made {
            self.release(&f);
        }
        result
    }
}
