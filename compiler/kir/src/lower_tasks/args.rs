// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The parts of a spawned call worked out before its task starts (spec
//! 9.1): its arguments, and the value a method is called on, as the
//! checker found them (`kavach_checker::spawn_parts`). The task gets their
//! values, not the variables they were made from: `spawn cores
//! work_on(r.flag, 5)` gave the task all of `r` (Sakshi's S10), and
//! `spawn cores total(xs.slice(a, b))` all of `xs`.

use crate::lower::Lowerer;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// The parts of spawned `x` worked out before its task, in the order
    /// they are written: the value a method is called on or the function
    /// worked out from an expression, then the arguments.
    pub(crate) fn task_parts(&self, x: &'a ast::Expr) -> Vec<&'a ast::Expr> {
        let ast::ExprKind::Call { callee, args, .. } = &x.kind else {
            return Vec::new();
        };
        // Native code leaves `debug(…)` out, its argument too (spec 1.10).
        if self.is_builtin(callee, "debug") {
            return Vec::new();
        }
        // The value before the dot, or a function worked out from an
        // expression; the checker leaves out a plain name.
        let receiver = match &callee.kind {
            ast::ExprKind::Field { base, .. } => Some(&**base),
            _ => Some(&**callee),
        };
        receiver
            .into_iter()
            .chain(args.iter().map(|a| &a.value))
            .filter(|p| self.c.spawn_parts.contains(&(p.span.start, p.span.end)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{ExprKind, Type};

    /// Sakshi's S10: the task of `spawn cores work_on(r.flag, 5)` holds a
    /// `Bool`, not the record `r`.
    #[test]
    fn a_spawned_call_takes_only_the_field() {
        let src = "type R {\n    flag: Bool\n    items: [Int]\n}\n\nfn work_on(f: Bool, n: Int) -> Int {\n    if f { n } else { 0 }\n}\n\nfn main() {\n    const r = R(flag: true, items: [1, 2])\n    const t = spawn cores work_on(r.flag, 5)\n    print(t.wait() catch 0)\n}\n";
        let c = kavach_driver::compile(src, "t.kav", Default::default());
        let program = crate::lower(&c).expect("lowers");
        let mut held = Vec::new();
        for f in &program.functions {
            crate::walk::any_in_block(&f.body, &mut |e| {
                if let ExprKind::Rt(crate::rt_op::RtOp::TaskSpawn, args) = &e.kind
                    && let ExprKind::Closure { captures, .. } = &args[0].kind
                {
                    held.extend(captures.iter().map(|c| c.ty));
                }
                false
            });
        }
        assert_eq!(held, vec![Type::Bool]);
    }
}
