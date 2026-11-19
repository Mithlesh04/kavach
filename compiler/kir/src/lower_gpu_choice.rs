// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Choosing a GPU (spec 15.6). `xs.map(f, on: device)` runs its `map`
//! inside a `GpuPlace` scope, as `gpu.on(device) { … }` and `on gpu(n)
//! x` do (`lower_scoped.rs`); the runtime picks the device from the
//! innermost one. A `gpu(n) fn` given to a `map` puts a `// gpu n` line
//! before its kernel, which the `wgsl` backend keeps. The program's start
//! checks that every device the code names is there (`GpuStart`).

use crate::ir::{self, ExprKind as K, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast as ast;

impl<'a> Lowerer<'a> {
    /// The number of the `GpuDevice` `device`: its `index` field.
    pub(crate) fn device_number(&mut self, device: &'a ast::Expr) -> Lowered<ir::Expr> {
        let value = self.expr(device)?;
        let (index, ty) = self.field_index(value.ty, "index", device.span)?;
        let line = self.line(device.span);
        Ok(self.at(K::GetField(Box::new(value), index), ty, line))
    }

    /// `xs.map(f, on: device)`: the `map` inside a place with its GPU
    /// work on `device`; `None` when the call has no `on:`.
    pub(crate) fn map_on_device(
        &mut self,
        base: &'a ast::Expr,
        name: &'a ast::Ident,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        call: &'a ast::Expr,
    ) -> Option<Lowered<ir::Expr>> {
        let (last, rest) = args.split_last()?;
        if name.name != "map" || last.name.as_ref().is_none_or(|n| n.name != "on") {
            return None;
        }
        let device = match self.device_number(&last.value) {
            Ok(d) => d,
            Err(e) => return Some(Err(e)),
        };
        let start = K::Rt(RtOp::GpuPlace, vec![device]);
        Some(self.within(start, ty, line, |me, _| {
            me.method_call(base, name, rest, ty, line, call).map(Some)
        }))
    }

    /// The `// gpu n` line for the kernel of a `map`, `filter` or `sum`
    /// given `f`: the number of the `gpu(n) fn` it is, or calls, if any.
    pub(crate) fn kernel_device_line(&self, f: &ast::Expr) -> String {
        let mut found = None;
        self.gpu_fn_number(f, true, &mut found);
        found.map_or(String::new(), |n| format!("// gpu {n}\n"))
    }

    /// The number of a `gpu(n) fn` that `e` names (`top`: as given to
    /// the `map`) or calls.
    fn gpu_fn_number(&self, e: &ast::Expr, top: bool, found: &mut Option<u64>) {
        let named = |callee: &ast::Expr| {
            let ast::ExprKind::Name(id) = &callee.kind else {
                return None;
            };
            let def = self.c.resolution.uses.get(&id.span.start)?;
            let fid = self.fn_ids.get(def)?;
            self.fns[*fid as usize].0?.gpu_device
        };
        match &e.kind {
            ast::ExprKind::Name(_) if top => *found = found.or(named(e)),
            ast::ExprKind::Lambda { body, .. } if top => self.gpu_fn_number(body, false, found),
            ast::ExprKind::Call { callee, args, .. } => {
                *found = found.or(named(callee));
                for a in args {
                    self.gpu_fn_number(&a.value, false, found);
                }
            }
            ast::ExprKind::Unary(_, x) => self.gpu_fn_number(x, false, found),
            ast::ExprKind::Binary(_, a, b) => {
                self.gpu_fn_number(a, false, found);
                self.gpu_fn_number(b, false, found);
            }
            ast::ExprKind::Tuple(items) => {
                for x in items {
                    self.gpu_fn_number(x, false, found);
                }
            }
            _ => {}
        }
    }

    /// `GpuStart`, for the start of the entry function, when the code
    /// names a GPU by number or `[run] gpu` names one (spec 15.6).
    pub(crate) fn gpu_start(&mut self, file_setting: Option<&str>) -> Option<Stmt> {
        let (highest, place) = self.highest_gpu();
        let device_setting = file_setting.filter(|t| !matches!(*t, "cpu" | "auto"));
        if highest.is_none() && device_setting.is_none() {
            return None;
        }
        let at = |kind, ty| ir::Expr { kind, ty, line: 1 };
        let str_ty = self.str_type();
        let text = |t: &str| at(K::Str(t.to_string()), str_ty);
        let highest = at(K::Int(highest.map_or(-1, i128::from)), Type::I64);
        let operands = vec![highest, text(&place), text(device_setting.unwrap_or(""))];
        Some(Stmt::Expr(at(K::Rt(RtOp::GpuStart, operands), Type::Unit)))
    }

    /// The highest GPU number the code writes, in a `gpu(n) fn` or an
    /// `on gpu(n)` with a number, and where.
    fn highest_gpu(&self) -> (Option<u64>, String) {
        match ast::highest_gpu(&self.c.file) {
            Some((n, ast::GpuNumberAt::Fn(name))) => (Some(n), format!("`gpu({n}) fn {name}`")),
            Some((n, ast::GpuNumberAt::On(span))) => {
                let line = self.line(span);
                (Some(n), format!("`on gpu({n})` on line {line}"))
            }
            None => (None, String::new()),
        }
    }
}
