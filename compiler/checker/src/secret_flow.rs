// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Which variables hold a secret (spec 8.7). A variable given a secret,
//! when it is made or later (`=`, `+=`, `xs[i] =`), holds one everywhere in
//! its function, before that line too: a loop may come back to it. So
//! each variable's secrecy is settled once the whole file is checked, and
//! the places a variable went that a secret may not are reported then.

use crate::checker::Checker;
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct Flow {
    /// Calls whose arguments were checked against their parameters.
    pub(crate) checked: HashSet<(u32, u32)>,
    /// Each variable given a value: whether the value was a secret
    /// already, and the variables it was made from.
    gets: Vec<(DefId, bool, Vec<DefId>)>,
    /// Where a variable went that a secret may not, and the message if
    /// it holds one.
    pub(crate) reaches: Vec<(DefId, Span, String)>,
    /// The parameters of the lambdas being checked, innermost last.
    pub(crate) own: Vec<Vec<DefId>>,
    /// `inout` parameters: what they are given goes back to the caller.
    pub(crate) inout: HashSet<DefId>,
}

impl Checker<'_> {
    /// Variable `def` is given `value`.
    pub(crate) fn flow_into(&mut self, def: DefId, value: &Expr) {
        let hits = self.secrets_in(value);
        let now = hits.iter().any(|h| h.var.is_none());
        let from = hits.iter().filter_map(|h| h.var).collect();
        self.secret_flow.gets.push((def, now, from));
    }

    /// The names `pattern` binds are given parts of `value`.
    pub(crate) fn flow_pattern(&mut self, pattern: &Pattern, value: &Expr) {
        if let (PatternKind::Tuple(ps), ExprKind::Tuple(xs)) = (&pattern.kind, &value.kind)
            && ps.len() == xs.len()
        {
            for (p, x) in ps.iter().zip(xs) {
                self.flow_pattern(p, x);
            }
            return;
        }
        for def in self.bound(pattern) {
            self.flow_into(def, value);
        }
    }

    /// `if let` and `match` give their names parts of the value matched.
    pub(crate) fn flow_branches(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::IfLet { name, value, .. } => {
                if let Some(&def) = self.decl.get(&name.span.start) {
                    self.flow_into(def, value);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                for arm in arms {
                    self.flow_pattern(&arm.pattern, scrutinee);
                }
            }
            _ => {}
        }
    }

    /// `target = value`, or `target op= value`.
    pub(crate) fn flow_assign(&mut self, target: &Expr, value: &Expr) {
        match (&target.kind, &value.kind) {
            (ExprKind::Tuple(ts), ExprKind::Tuple(xs)) if ts.len() == xs.len() => {
                ts.iter().zip(xs).for_each(|(t, x)| self.flow_assign(t, x))
            }
            (ExprKind::Tuple(ts), _) => ts.iter().for_each(|t| self.flow_assign(t, value)),
            _ => self.flow_assign_one(target, value),
        }
    }

    fn flow_assign_one(&mut self, target: &Expr, value: &Expr) {
        let here = (target.span.start, target.span.end);
        match &target.kind {
            ExprKind::Name(id) => {
                if let Some(&def) = self.res.uses.get(&id.span.start) {
                    self.flow_into(def, value);
                }
            }
            // As when the record is made: only a `secret` field takes one.
            ExprKind::Field { name, .. } if !self.secret.contains(&here) => {
                for hit in self.secrets_in(value) {
                    let message = format!(
                        "`{}` is secret, so it can't go into the field `{}`, which isn't `secret`; mark it `secret`, or use `reveal(…)` if you really mean to",
                        hit.name, name.name
                    );
                    self.report_secret(hit, message);
                }
            }
            ExprKind::Field { .. } => {}
            ExprKind::Index { base, index } => self.flow_into_root(base, &[value, index]),
            _ => self.flow_into_root(target, &[value]),
        }
    }

    /// A method's call: the lambdas given to it take parts of its
    /// receiver. Its other arguments are refused when they hold a secret,
    /// as any call's are (`calls.rs`).
    pub(crate) fn flow_call(&mut self, callee: &Expr, args: &[Arg]) {
        let ExprKind::Field { base, .. } = &callee.kind else {
            return;
        };
        for a in args {
            if let ExprKind::Lambda { params, .. } = &a.value.kind {
                for p in params {
                    if let Some(&def) = self.decl.get(&p.name.span.start) {
                        self.flow_into(def, base);
                    }
                }
            }
        }
    }

    /// `values` are stored in a part of `place`: its variable holds what
    /// they hold. Only a variable of this function may: a secret stored in
    /// `self` or an `inout` parameter would reach the caller unseen.
    fn flow_into_root(&mut self, place: &Expr, values: &[&Expr]) {
        let mut root = place;
        while let ExprKind::Index { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Field { base, .. } = &root.kind
        {
            // A part of a `secret` field may hold one.
            if self.secret.contains(&(root.span.start, root.span.end)) {
                return;
            }
            root = base;
        }
        let def = match &root.kind {
            ExprKind::Name(id) => self.res.uses.get(&id.span.start).copied(),
            _ => None,
        };
        match def.filter(|d| !self.secret_flow.inout.contains(d)) {
            Some(def) => values.iter().for_each(|x| self.flow_into(def, x)),
            None => {
                for x in values {
                    for hit in self.secrets_in(x) {
                        let message = format!(
                            "`{}` is secret, so it can't be stored here: only a variable of this function or a `secret` field holds one; use `reveal(…)` if you really mean to",
                            hit.name
                        );
                        self.report_secret(hit, message);
                    }
                }
            }
        }
    }

    /// Settles which variables hold a secret, then reports where those
    /// went that a secret may not.
    pub(crate) fn check_secrets_later(&mut self) {
        let gets = std::mem::take(&mut self.secret_flow.gets);
        loop {
            let before = self.secret_defs.len();
            for (def, now, from) in &gets {
                if *now || from.iter().any(|d| self.secret_defs.contains(d)) {
                    self.secret_defs.insert(*def);
                }
            }
            if self.secret_defs.len() == before {
                break;
            }
        }
        let mut told = HashSet::new();
        for (def, span, message) in std::mem::take(&mut self.secret_flow.reaches) {
            if self.secret_defs.contains(&def) && told.insert((span.start, message.clone())) {
                self.error(span, message);
            }
        }
    }

    /// The variables `p` binds, at any depth.
    pub(crate) fn bound(&self, p: &Pattern) -> Vec<DefId> {
        let mut names = Vec::new();
        names_in(p, &mut names);
        (names.into_iter())
            .filter_map(|id| self.decl.get(&id.span.start).copied())
            .collect()
    }
}

fn names_in<'p>(p: &'p Pattern, out: &mut Vec<&'p Ident>) {
    match &p.kind {
        PatternKind::Binding(id) => out.push(id),
        PatternKind::Tuple(ps) => ps.iter().for_each(|x| names_in(x, out)),
        PatternKind::Variant {
            fields: Some(fs), ..
        } => fs.iter().for_each(|f| names_in(&f.pattern, out)),
        PatternKind::Ok(x) | PatternKind::Err(x) => names_in(x, out),
        PatternKind::List { items, rest } => {
            items.iter().for_each(|it| names_in(&it.pattern, out));
            out.extend(rest.iter());
        }
        PatternKind::Record(fields) => fields.iter().for_each(|f| names_in(&f.pattern, out)),
        _ => {}
    }
}
