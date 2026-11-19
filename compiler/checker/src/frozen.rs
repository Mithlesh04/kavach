// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Frozen values (spec 2.20): `x.freeze()`, `Frozen<T>` and
//! `f.unfreeze()`. Reading sees through `Frozen` (`Infer::shallow`); here
//! are the calls, the changes refused, and where a frozen value may go
//! where a plain `T` is written. `frozen_reads.rs` works out which
//! functions only read a parameter.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::{HashMap, HashSet};

/// A call of the built-in `freeze` or `unfreeze`, for code generation:
/// `true` when it has a `key:`. A value frozen without a key is the plain
/// value while running; a sealed one is `(value, key)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrozenCall {
    Freeze(bool),
    Unfreeze(bool),
}

/// The panic of `f.unfreeze(key: k)` with a key not equal to the one `f`
/// was sealed with (spec 2.20).
pub const WRONG_KEY: &str =
    "this key isn't the one the value was sealed with, so `unfreeze` can't open it (spec 2.20)";

/// What the arguments being checked go to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Callee {
    /// Somewhere a plain `T` is a value of its own: a record's field, a
    /// changing method's argument.
    #[default]
    Keeps,
    /// A built-in method that only reads its arguments.
    Reads,
    /// A function or method of the program: only if it only reads that
    /// parameter.
    Fn(DefId),
}

/// A frozen value given where a `T` is written, to a function's
/// parameter (`None` for `self`): checked once every body is known.
#[derive(Clone, Debug)]
pub(crate) struct Passed {
    pub(crate) span: Span,
    pub(crate) callee: DefId,
    pub(crate) param: Option<usize>,
    pub(crate) value: String,
    pub(crate) sealed: bool,
}

#[derive(Default)]
pub(crate) struct State {
    /// Whether the program has a frozen value at all.
    pub(crate) used: bool,
    pub(crate) calls: HashMap<(u32, u32), FrozenCall>,
    /// Set just before `args_with` by its caller; taken at its start.
    pub(crate) callee: Callee,
    /// The argument being fitted to its parameter: its callee, index and
    /// span, and how it is named in a message.
    pub(crate) at: Option<(Callee, usize, Span)>,
    pub(crate) at_name: String,
    pub(crate) passed: Vec<Passed>,
    /// Sealed values read as their `T`: code generation takes the value
    /// out of its `(value, key)` there.
    pub(crate) reads: HashSet<(u32, u32)>,
}

/// How a value is named in a message: `order`, `order.lines`, or `this value`.
pub(crate) fn shown(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Name(id) => id.name.clone(),
        ExprKind::SelfValue => "self".to_string(),
        ExprKind::Field { base, name, .. } => match &base.kind {
            ExprKind::Name(_) | ExprKind::SelfValue | ExprKind::Field { .. } => {
                format!("{}.{}", shown(base), name.name)
            }
            _ => "this value".to_string(),
        },
        _ => "this value".to_string(),
    }
}

/// `x.unfreeze()`, or with the key when the value is sealed.
fn unfreeze_of(name: &str, sealed: bool) -> String {
    match (name, sealed) {
        ("this value", false) => "`unfreeze()`".to_string(),
        ("this value", true) => "`unfreeze(key: …)`".to_string(),
        (n, false) => format!("`{n}.unfreeze()`"),
        (n, true) => format!("`{n}.unfreeze(key: …)`"),
    }
}

impl Checker<'_> {
    /// `x.freeze(…)` and `f.unfreeze(…)`, the built-in methods of every value.
    pub(crate) fn frozen_method(
        &mut self,
        e: &Expr,
        base: &Expr,
        name: &Ident,
        args: &[Arg],
    ) -> Option<Ty> {
        let unfreeze = match name.name.as_str() {
            "freeze" => false,
            "unfreeze" => true,
            _ => return None,
        };
        let recv = self.value(base, None);
        self.frozen.used = true;
        let (mut key, mut nested) = (None, true);
        for a in args {
            match a.name.as_ref().map(|n| n.name.as_str()) {
                Some("key") => key = Some(&a.value),
                Some("nested") if !unfreeze => match a.value.kind {
                    ExprKind::Bool(b) => nested = b,
                    _ => {
                        self.error(a.value.span, "`nested:` is written `true` or `false`, as it is part of the frozen value's type (spec 2.20)");
                        self.value(&a.value, None);
                    }
                },
                _ => {
                    let takes = match unfreeze {
                        true => "`unfreeze` takes only `key:`",
                        false => "`freeze` takes only `nested:` and `key:`",
                    };
                    self.error(a.value.span, format!("{takes} (spec 2.20)"));
                    self.value(&a.value, None);
                }
            }
        }
        let span = (e.span.start, e.span.end);
        let who = match shown(base).as_str() {
            "this value" => "this value".to_string(),
            name => format!("`{name}`"),
        };
        let (plain, sealed_fix) = (
            unfreeze_of(&shown(base), false),
            unfreeze_of(&shown(base), true),
        );
        if !unfreeze {
            if let Ty::Frozen(..) = self.inf.shallow_keep(&recv) {
                self.error(name.span, format!("{who} is already frozen (spec 2.20)"));
            }
            let key = key.map(|k| Box::new(self.value(k, None)));
            self.frozen
                .calls
                .insert(span, FrozenCall::Freeze(key.is_some()));
            return Some(Ty::Frozen(Box::new(recv), nested, key));
        }
        let (inner, sealed) = match self.inf.shallow_keep(&recv) {
            Ty::Frozen(inner, _, sealed) => (*inner, sealed),
            other => {
                if !matches!(other, Ty::Error | Ty::Var(_) | Ty::Never) {
                    self.error(
                        name.span,
                        format!("{who} is not frozen: `unfreeze()` gives a copy of a `Frozen<T>` that can change (spec 2.20)"),
                    );
                }
                if let Some(k) = key {
                    self.value(k, None);
                }
                return Some(other);
            }
        };
        match (sealed, key) {
            (None, Some(k)) => {
                self.error(k.span, format!("{who} isn't sealed with a key: write {plain} (spec 2.20)"));
                self.value(k, None);
            }
            (Some(_), None) => self.error(
                name.span,
                format!("{who} is sealed with a key: only {sealed_fix}, with the key it was frozen with, opens it (spec 2.20)"),
            ),
            (Some(want), Some(k)) => {
                self.check_expect(k, &want);
                self.check_key_named(&want, e.span);
            }
            (None, None) => {}
        }
        self.frozen
            .calls
            .insert(span, FrozenCall::Unfreeze(key.is_some()));
        Some(inner)
    }

    /// A key whose type is private to another module: only that module
    /// can name it, so only it unfreezes (spec 2.20).
    fn check_key_named(&mut self, key: &Ty, at: Span) {
        let Ty::Adt(def, _) = self.inf.resolve(key) else {
            return;
        };
        let Some(decl) = self.res.defs[def].span else {
            return;
        };
        let home = self.res.module_at(decl.start);
        if self.res.public.contains(&def) || home == self.res.module_at(at.start) {
            return;
        }
        let module = self.res.modules[home].name.clone();
        let name = self.def_name(def).to_string();
        self.error(
            at,
            format!("only module `{module}` can unfreeze this value: its key's type `{name}` is private there (spec 2.20)"),
        );
    }

    /// Whether a value of type `actual` fits where `expected` is, when
    /// either is frozen; `None` when neither is, or a type is unknown.
    pub(crate) fn coerce_frozen(&mut self, actual: &Ty, expected: &Ty) -> Option<bool> {
        let (a, e) = (
            self.inf.shallow_keep(actual),
            self.inf.shallow_keep(expected),
        );
        let open = |t: &Ty| matches!(t, Ty::Var(_) | Ty::Error | Ty::Never);
        match (&a, &e) {
            (Ty::Frozen(x, _, k1), Ty::Frozen(y, _, k2)) => Some(match (k1, k2) {
                (None, None) => self.inf.unify(x, y),
                (Some(k1), Some(k2)) => self.inf.unify(x, y) && self.inf.unify(k1, k2),
                _ => false,
            }),
            // Worked out from here: the frozen type (spec 2.20).
            (Ty::Frozen(..), e) if open(e) => Some(self.inf.unify(&a, e)),
            (Ty::Frozen(..), Ty::Optional(y))
                if matches!(self.inf.shallow_keep(y), Ty::Frozen(..)) || open(y) =>
            {
                Some(self.inf.unify(&a, y))
            }
            (Ty::Frozen(inner, _, key), _) => {
                let Some((callee, index, span)) = self.frozen.at else {
                    return Some(false);
                };
                match callee {
                    Callee::Keeps => return Some(false),
                    Callee::Reads => {}
                    Callee::Fn(def) => self.frozen.passed.push(Passed {
                        span,
                        callee: def,
                        param: Some(index),
                        value: self.frozen.at_name.clone(),
                        sealed: key.is_some(),
                    }),
                }
                if key.is_some() {
                    self.frozen.reads.insert((span.start, span.end));
                }
                let inner = (**inner).clone();
                Some(self.coerce(&inner, expected))
            }
            (a, Ty::Frozen(..)) if !open(a) => Some(false),
            _ => None,
        }
    }

    /// The message for a value that doesn't fit, when one side is frozen.
    pub(crate) fn frozen_mismatch(&self, e: &Expr, expected: &Ty, actual: &Ty) -> Option<String> {
        let (want, got) = (self.show(expected), self.show(actual));
        match (
            self.inf.shallow_keep(actual),
            self.inf.shallow_keep(expected),
        ) {
            (Ty::Frozen(..), Ty::Frozen(..)) => None,
            (Ty::Frozen(_, _, key), _) => {
                let name = shown(e);
                let fix = unfreeze_of(&name, key.is_some());
                let name = match name.as_str() {
                    "this value" => "this value".to_string(),
                    n => format!("`{n}`"),
                };
                Some(format!(
                    "{name} is a `{got}`, which never changes, so it can't go where a `{want}` is: call {fix} for a copy that can change (spec 2.20)"
                ))
            }
            (_, Ty::Frozen(..)) => Some(format!(
                "expected `{want}`, found `{got}`: call `.freeze()` on the value to make one (spec 2.20)"
            )),
            _ => None,
        }
    }

    /// `target` changes (`how`): as a slot of the value that holds it when
    /// `slot` (assigned, or passed `inout`), or in place (`push`).
    /// `whole` when it is given a new value with `=`. An error when it is
    /// frozen, or inside a frozen value: all the way down, or only as one
    /// of its own fields and items when frozen with `nested: false`
    /// (spec 2.20).
    pub(crate) fn check_frozen_target(
        &mut self,
        target: &Expr,
        how: &str,
        slot: bool,
        whole: bool,
    ) {
        if !self.frozen.used {
            return;
        }
        let mut chain = vec![target];
        let mut at = target;
        while let ExprKind::Field { base, .. }
        | ExprKind::Index { base, .. }
        | ExprKind::TupleField { base, .. } = &at.kind
        {
            chain.push(base);
            at = base;
        }
        for (i, part) in chain.iter().enumerate() {
            let Some(t) = self.type_at(part.span) else {
                continue;
            };
            let Ty::Frozen(_, deep, key) = self.inf.shallow_keep(&t) else {
                continue;
            };
            let refused = match i {
                0 => !whole,
                1 => slot || deep,
                _ => deep,
            };
            if refused {
                let name = shown(part);
                let fix = unfreeze_of(&name, key.is_some());
                let down = if i > 1 { ", all the way down" } else { "" };
                self.error(
                    target.span,
                    format!("`{name}` is frozen{down}, so it can't be {how}: call {fix} for a copy that can change (spec 2.20)"),
                );
                return;
            }
        }
    }

    /// `target op= v`: assigned, or changed with `op=` (spec 2.20).
    pub(crate) fn check_frozen_assign(&mut self, target: &Expr, op: AssignOp) {
        let how = match op.binary() {
            Some(bin) => format!("changed with `{}=`", bin.as_str()),
            None => "assigned".to_string(),
        };
        self.check_frozen_target(target, &how, true, op == AssignOp::Set);
    }

    /// `x`, compared with a value of type `t`: reading, so a frozen `x`
    /// fits a plain `t` (spec 2.20).
    pub(crate) fn expect_read(&mut self, x: &Expr, t: &Ty) {
        let actual = self.value(x, Some(t));
        self.fitting = Some(self.made(x));
        self.frozen.at = Some((Callee::Reads, 0, x.span));
        let fits = self.coerce(&actual, t);
        self.frozen.at = None;
        self.fitting = None;
        if !fits {
            self.mismatch(x, t, &actual);
        }
    }

    /// The type a `match` on a value of type `t` gives its patterns: a
    /// frozen value's own type, read; an arm that names the whole value
    /// keeps it frozen. A sealed value can't do both in one `match`, as
    /// a name would hold it without its key (spec 2.20).
    pub(crate) fn frozen_scrutinee(&mut self, t: &Ty, arms: &[MatchArm], at: Span) -> Ty {
        let Ty::Frozen(inner, _, key) = self.inf.shallow_keep(t) else {
            return t.clone();
        };
        let named = arms
            .iter()
            .filter(|a| matches!(a.pattern.kind, PatternKind::Binding(_)))
            .count();
        if key.is_some() && named > 0 && named < arms.len() {
            self.error(at, "a sealed value's `match` either reads its parts or names the whole value, not both: name it in every arm, or unfreeze it first (spec 2.20)");
        }
        *inner
    }

    /// `f.total()` on a frozen value: the method's `self` must only be
    /// read, as a parameter must (spec 2.20).
    pub(crate) fn note_frozen_self(&mut self, base: &Expr, recv: &Ty, method: Option<DefId>) {
        let (Some(def), Ty::Frozen(_, _, key)) = (method, self.inf.shallow_keep(recv)) else {
            return;
        };
        self.frozen.passed.push(Passed {
            span: base.span,
            callee: def,
            param: None,
            value: shown(base),
            sealed: key.is_some(),
        });
    }
}
