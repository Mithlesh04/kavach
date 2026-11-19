// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
        // A C function ending in `...` is called by its whole type.
        let ty = (self.m.c_variadic[f].as_ref()).map_or(String::new(), |p| format!(" ({p})"));
        let call = format!("notail call {ret}{ty} @{name}({})", list.join(", "));
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

    /// A call of the C function at address `args[0]` with the other
    /// operands, giving `ty`: a library loaded while running (spec 10.7).
    pub(crate) fn call_address(&mut self, ty: Type, args: &[Expr]) -> Option<String> {
        let address = self.expr(&args[0])?;
        let mut list = Vec::new();
        for a in &args[1..] {
            if let Some(v) = self.expr(a) {
                list.push(format!("{} {v}", llvm_type(a.ty)));
            }
        }
        let target = self.val(format!("inttoptr i64 {address} to ptr"));
        let ret = crate::module::ret_type(ty, false);
        let call = format!("call {ret} {target}({})", list.join(", "));
        if ret == "void" {
            self.ins(call);
            None
        } else {
            Some(self.val(call))
        }
    }

    /// A call into C with records by value (`RtOp::CallC`), as
    /// `args[0]` says (`kavach_kir::c_abi::descriptor`): `args[1]` is the
    /// function's address; result parts are stored where `m` points.
    pub(crate) fn call_c(&mut self, ty: Type, args: &[Expr]) -> Option<String> {
        let ExprKind::Str(text) = &args[0].kind else {
            unreachable!("a C call's descriptor is a text constant")
        };
        let (result, words) = text.split_once('|').unwrap_or(("v", ""));
        let address = self.expr(&args[1])?;
        let target = self.val(format!("inttoptr i64 {address} to ptr"));
        let (mut list, mut out) = (Vec::new(), None);
        for (word, a) in words.split(',').filter(|w| !w.is_empty()).zip(&args[2..]) {
            let v = self.expr(a)?;
            let size = &word[1..];
            list.push(match &word[..1] {
                "m" => {
                    out = Some(v);
                    continue;
                }
                "i" => format!("i64 {v}"),
                "f" => format!("double {v}"),
                kind @ ("s" | "h") => {
                    let p = self.val(format!("inttoptr i64 {v} to ptr"));
                    let how = if kind == "s" { "byval" } else { "sret" };
                    format!("ptr {how}([{size} x i8]) align 8 {p}")
                }
                _ => format!("{} {v}", llvm_type(a.ty)),
            });
        }
        let part = |c: char| if c == 'f' { "double" } else { "i64" };
        let ret = match result {
            "v" => "void".to_string(),
            "n" => crate::module::ret_type(ty, false),
            p if p.len() == 1 => part(p.chars().next().unwrap_or('i')).to_string(),
            p => format!(
                "{{ {} }}",
                p.chars().map(part).collect::<Vec<_>>().join(", ")
            ),
        };
        let call = format!("call {ret} {target}({})", list.join(", "));
        if ret == "void" {
            self.ins(call);
            return None;
        }
        let r = self.val(call);
        let Some(out) = out else {
            return Some(r);
        };
        let one = result.len() == 1;
        for (k, c) in result.chars().enumerate() {
            let v = if one {
                r.clone()
            } else {
                self.val(format!("extractvalue {ret} {r}, {k}"))
            };
            let at = self.val(format!("add i64 {out}, {}", 8 * k));
            let p = self.val(format!("inttoptr i64 {at} to ptr"));
            self.ins(format!("store {} {v}, ptr {p}, align 1", part(c)));
        }
        None
    }

    /// A loop on all cores (decision 0031): `kv_par_for`, the reduction's
    /// `kv_par_for_red` or the grid rows' `kv_par_for_rows`, with the
    /// chunk function's address; the runtime gives 1 for "run the plain
    /// loop yourself".
    pub(crate) fn par_for(&mut self, op: kavach_kir::RtOp, args: &[Expr]) -> Option<String> {
        let lo = self.expr(&args[0])?;
        let hi = self.expr(&args[1])?;
        let ExprKind::Int(chunk) = args[2].kind else {
            unreachable!("the chunk's number is a constant")
        };
        let cores = self.expr(&args[3])?;
        // Rows: their width comes next, then the captures.
        let rows = op == kavach_kir::RtOp::ParForRows;
        let width = match rows {
            true => Some(self.expr(&args[4])?),
            false => None,
        };
        let first_cap = if rows { 5 } else { 4 };
        let caps = self.slot_array(&args[first_cap..]);
        let name = fn_name(chunk as usize);
        let rt = match op {
            kavach_kir::RtOp::ParForRed => "kv_par_for_red",
            kavach_kir::RtOp::ParForRows => "kv_par_for_rows",
            _ => "kv_par_for",
        };
        let mut list = vec![
            ("i64", lo),
            ("i64", hi),
            ("ptr", format!("@{name}")),
            ("ptr", caps),
            ("i64", (args.len() - first_cap).to_string()),
            ("i64", cores),
        ];
        if let Some(w) = width {
            list.push(("i64", w));
        }
        Some(self.call_rt(rt, "i64", &list, false))
    }
}
