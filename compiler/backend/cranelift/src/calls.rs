//! Calls: of program functions and through function values. A closure's
//! function borrows its closure (`kavach_kir::borrows`): the caller keeps
//! its reference while the call runs, and lets go of a closure it made
//! just for the call afterwards.

use crate::codegen::FnGen;
use cranelift_codegen::ir::{InstBuilder, Value};
use kavach_kir::{Expr, ExprKind};

impl FnGen<'_, '_> {
    /// A closure for a call that borrows it (`kavach_kir::borrows`), and
    /// whether it was made for the call (then it is let go of after): a
    /// local is read in place unless an argument gives it a new value.
    pub(crate) fn callee(&mut self, c: &Expr, args: &[Expr]) -> Option<(Value, bool)> {
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

    /// The values of the arguments of a call of program function `f`: a
    /// closure's function borrows its closure, which is also given back
    /// when it was made for the call, to be let go of after it.
    pub(crate) fn call_args(
        &mut self,
        f: u32,
        args: &[Expr],
    ) -> Option<(Vec<Value>, Option<Value>)> {
        let mut values = Vec::new();
        let mut made = None;
        let rest = match args.split_first() {
            Some((c, rest)) if self.closures[f as usize] => {
                let (v, owned) = self.callee(c, rest)?;
                values.push(v);
                made = owned.then_some(v);
                rest
            }
            _ => args,
        };
        values.extend(rest.iter().filter_map(|a| self.expr(a)));
        Some((values, made))
    }

    /// A call through function value `callee`: the closure goes in as the
    /// function's first argument, which its function borrows. Its results:
    /// the value, and the error too when it `fails`.
    pub(crate) fn call_value(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        ty: kavach_kir::Type,
        fails: bool,
        line: u32,
    ) -> Option<Vec<Value>> {
        // The closure is borrowed (`kavach_kir::borrows`): let go of after
        // the call when it was made for it.
        let (f, made) = self.callee(callee, args)?;
        let addr = self.field(f, 0);
        let mut values = vec![f];
        values.extend(args.iter().filter_map(|a| self.expr(a)));
        let mut params = vec![callee.ty];
        params.extend(args.iter().map(|a| a.ty));
        let sig = crate::abi::signature(&*self.module, &params, ty, fails);
        let sig = self.b.import_signature(sig);
        self.check_stack(line);
        let call = self.b.ins().call_indirect(sig, addr, &values);
        let results = self.b.inst_results(call).to_vec();
        if made {
            self.release(f);
        }
        Some(results)
    }
}
