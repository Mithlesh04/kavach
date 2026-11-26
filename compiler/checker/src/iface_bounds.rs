// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Interface bounds at a call (spec 2.9): `show<T: Describe>(x)` needs
//! the type given for `T` to implement `Describe`. A type parameter
//! meets the bound when its own bounds include it; a type not known yet
//! is checked once it is known, after the bodies (split from `calls.rs`).

use crate::checker::Checker;
use crate::infer::VarKind;
use crate::ty::{Prim, Ty};
use kavach_ast::Span;
use kavach_resolver::DefId;
use std::collections::HashSet;

impl Checker<'_> {
    /// Checks that `t`, given at a call for a type parameter with the
    /// interface `bounds`, implements each. A type not known yet waits
    /// for `iface_bounds_later`.
    pub(crate) fn iface_bounds(&mut self, bounds: &[DefId], t: &Ty, span: Span) {
        if bounds.is_empty() {
            return;
        }
        if let Ty::Var(_) = self.inf.resolve(t) {
            self.iface_checks.push((bounds.to_vec(), t.clone(), span));
            return;
        }
        self.iface_bounds_now(bounds, t, span);
    }

    /// The bounds noted while their type was not known, now the bodies
    /// have said what it is.
    pub(crate) fn iface_bounds_later(&mut self) {
        for (bounds, t, span) in std::mem::take(&mut self.iface_checks) {
            // A literal with no type of its own is an `Int` or a `Float`.
            let t = match self.inf.resolve(&t) {
                Ty::Var(v) => match self.inf.kind(v) {
                    VarKind::IntLiteral => Ty::Prim(Prim::Int),
                    VarKind::FloatLiteral => Ty::Prim(Prim::Float),
                    // Still unknown: a message elsewhere says so.
                    VarKind::Any => continue,
                },
                t => t,
            };
            self.iface_bounds_now(&bounds, &t, span);
        }
    }

    fn iface_bounds_now(&mut self, bounds: &[DefId], t: &Ty, span: Span) {
        let ty = self.inf.resolve(t);
        let has: Option<HashSet<DefId>> = match &ty {
            Ty::Adt(def, _) => self.implements.get(def).cloned(),
            Ty::Prim(p) => self.ext_implements.get(p.name()).cloned(),
            // A function's type parameter has the bounds it was declared
            // with. A type's parameters keep none yet, so they aren't
            // checked here.
            Ty::Param(p) => match self.bounds.get(p) {
                Some(b) => Some(b.iter().copied().collect()),
                None => return,
            },
            Ty::Var(_) | Ty::Error | Ty::Never => return,
            _ => None,
        };
        let missing: Vec<_> = bounds
            .iter()
            .filter(|b| !has.as_ref().is_some_and(|s| s.contains(b)))
            .copied()
            .collect();
        for b in missing {
            let i = self.def_name(b).to_string();
            let (t, fix) = match &ty {
                Ty::Adt(def, _) => {
                    let t = self.def_name(*def).to_string();
                    let fix = format!(": write `type {t}: {i}` or `extend {t}: {i}`");
                    (t, fix)
                }
                // `log.info("order placed")`: events are records (14.12).
                Ty::Prim(_) if i == "LogEvent" => (
                    self.show(&ty),
                    ": log events are records, so a log line can be searched by its fields. Declare one, as `type OrderPlaced: LogEvent { order_id: Int }`, and log that: `log.info(OrderPlaced(order_id: 7))` (spec 14.12)".to_string(),
                ),
                Ty::Prim(_) => (
                    self.show(&ty),
                    format!(": write `extend {}: {i}`", self.show(&ty)),
                ),
                Ty::Param(_) => {
                    let t = self.show(&ty);
                    let fix = format!(": write `{t}: {i}` where `{t}` is declared (spec 2.9)");
                    (t, fix)
                }
                _ => (self.show(&ty), String::new()),
            };
            let message = format!("`{t}` does not implement `{i}`{fix}");
            // A lambda's body may be checked twice: one message.
            if !self
                .errors
                .iter()
                .any(|e| e.span == span && e.message == message)
            {
                self.error(span, message);
            }
        }
    }
}
