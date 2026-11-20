// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Choosing a GPU (spec 15.6): the device given to `gpu.on(device)
//! { … }` and to `xs.map(f, on: device)`, and the `map`s of a `gpu(n)
//! fn`. Each choice needs `uses Gpu`; a plain `gpu fn` and its `map`
//! need nothing.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::{Arg, Expr, ExprKind};

impl Checker<'_> {
    /// `device`, given to `owner` (`gpu.on` or `on:`): a `GpuDevice` of
    /// `std.gpu`. Choosing needs `Gpu`.
    pub(crate) fn check_gpu_device(&mut self, device: &Expr, owner: &str) {
        let wanted = self.res.std_type("GpuDevice");
        let expected = wanted.map(|d| Ty::Adt(d, Vec::new()));
        let t = self.value(device, expected.as_ref());
        let t = self.inf.resolve(&t);
        let fits = matches!(&t, Ty::Adt(d, _) if Some(*d) == wanted);
        // A number written there is still a literal's type variable.
        let unknown = match &t {
            Ty::Var(v) => self.inf.kind(*v) == crate::infer::VarKind::Any,
            t => matches!(t, Ty::Error),
        };
        if !fits && !unknown {
            let shown = self.show(&t);
            self.error(
                device.span,
                format!("{owner} takes a `GpuDevice`, one of `gpu.devices()`, not {} (spec 15.6); write `on gpu(n)` before the value to choose by number", crate::op_impls::a_shown(&shown)),
            );
        }
        self.note_cap("Gpu", device.span);
    }

    /// `xs.map(f, on: device)`: the device, checked, and the arguments
    /// without it, for the `map` itself. `on:` comes last.
    pub(crate) fn map_device<'e>(&mut self, args: &'e [Arg]) -> &'e [Arg] {
        let Some((last, rest)) = args.split_last() else {
            return args;
        };
        if last.name.as_ref().is_none_or(|n| n.name != "on") {
            return args;
        }
        self.check_gpu_device(&last.value, "`on:`");
        rest
    }

    /// The `gpu(n) fn`s that `f`, given to a `map`, `filter` or `sum`,
    /// runs: a map of one is a choice, which needs `Gpu`, and one map runs
    /// on one device, so `gpu(n) fn`s of two numbers are an error.
    pub(crate) fn check_gpu_fn_choice(&mut self, f: &Expr) {
        let mut found: Vec<(u64, String)> = Vec::new();
        self.gpu_devices_called(f, true, &mut found);
        let Some((n, name)) = found.first().cloned() else {
            return;
        };
        self.note_cap("Gpu", f.span);
        if let Some((m, other)) = found.iter().find(|(m, _)| *m != n) {
            self.error(
                f.span,
                format!("this runs `gpu({n}) fn {name}` and `gpu({m}) fn {other}`, and one `map` runs on one GPU: give both the same number, or map them apart (spec 15.6)"),
            );
        }
    }

    /// Each `gpu(n) fn` that `e` names (`top`: given to the `map` as it
    /// is) or calls, with its number and name.
    fn gpu_devices_called(&self, e: &Expr, top: bool, found: &mut Vec<(u64, String)>) {
        let named = |callee: &Expr, found: &mut Vec<(u64, String)>| {
            if let ExprKind::Name(id) = &callee.kind
                && let Some(def) = self.res.uses.get(&id.span.start)
                && let Some(&n) = self.gpu_devices.get(def)
            {
                found.push((n, id.name.clone()));
            }
        };
        match &e.kind {
            ExprKind::Name(_) if top => named(e, found),
            ExprKind::Lambda { body, .. } if top => self.gpu_devices_called(body, false, found),
            ExprKind::Call { callee, args, .. } => {
                named(callee, found);
                for a in args {
                    self.gpu_devices_called(&a.value, false, found);
                }
            }
            ExprKind::Unary(_, x) => self.gpu_devices_called(x, false, found),
            ExprKind::Binary(_, a, b) => {
                self.gpu_devices_called(a, false, found);
                self.gpu_devices_called(b, false, found);
            }
            ExprKind::Tuple(items) => {
                for x in items {
                    self.gpu_devices_called(x, false, found);
                }
            }
            _ => {}
        }
    }
}
