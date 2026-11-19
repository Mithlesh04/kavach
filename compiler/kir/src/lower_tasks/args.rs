// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The arguments of a spawned call worked out before its task starts
//! (Sakshi's S10): `spawn cores work_on(r.flag, 5)` gave the task all of
//! `r` to read `r.flag` from, copying a big record for each task. A field
//! path can't fail, panic or change anything, so reading it first gives
//! the same result, and the task gets only the field's value.

use crate::lower::Lowerer;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// The arguments of `x`, a call of a program function, that are a
    /// field path of a variable (`r.flag`, `a.b.c`). None when the
    /// function takes an `inout` parameter, whose value goes back.
    pub(crate) fn task_parts(&self, x: &'a ast::Expr) -> Vec<&'a ast::Expr> {
        let ast::ExprKind::Call { callee, args, .. } = &x.kind else {
            return Vec::new();
        };
        let ast::ExprKind::Name(id) = &callee.kind else {
            return Vec::new();
        };
        let fid = (self.c.resolution.uses.get(&id.span.start)).and_then(|d| self.fn_ids.get(d));
        let Some(&fid) = fid else {
            return Vec::new();
        };
        if self.decl_of(fid).params.iter().any(|p| p.is_inout) {
            return Vec::new();
        }
        args.iter()
            .map(|a| &a.value)
            .filter(|v| self.field_path(v))
            .collect()
    }

    /// Whether `e` is `name.field`, or a longer path of fields, on a
    /// variable of the function being lowered.
    fn field_path(&self, e: &ast::Expr) -> bool {
        let ast::ExprKind::Field {
            base,
            optional: false,
            ..
        } = &e.kind
        else {
            return false;
        };
        match &base.kind {
            ast::ExprKind::Name(id) => (self.c.resolution.uses.get(&id.span.start))
                .is_some_and(|d| self.local_ids.contains_key(d)),
            ast::ExprKind::Field { .. } => self.field_path(base),
            _ => false,
        }
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
