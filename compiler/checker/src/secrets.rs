// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Secrets (spec 2.11, 8.7): where a secret value may go, and where it
//! may not: text, prints, parameters and results that aren't `secret`.
//! Which variables hold one is in `secret_flow.rs`.

use crate::checker::Checker;
use kavach_ast::*;
use kavach_resolver::DefId;

impl Checker<'_> {
    /// Whether `e` holds a secret, anywhere in it (spec 2.11, 8.7): `into`
    /// is where it goes, which may not take one.
    pub(crate) fn check_secret(&mut self, e: &Expr, into: &str) {
        let into = if into == "text" {
            "text".to_string()
        } else {
            format!("`{into}`")
        };
        for hit in self.secrets_in(e) {
            let message = format!(
                "`{}` is secret, so it can't go into {into}; use `reveal(…)` if you really mean to",
                hit.name
            );
            self.report_secret(hit, message);
        }
    }

    /// Passing a secret where nothing may receive it: an argument of a
    /// parameter, or a field, that isn't marked `secret` (spec 8.7).
    pub(crate) fn check_secret_arg(&mut self, e: &Expr, what: &str, name: &str, owner: &str) {
        for hit in self.secrets_in(e) {
            let message = format!(
                "`{}` is secret, so it can't go into the {what} `{name}` of {owner}, which isn't `secret`; mark it `secret`, or use `reveal(…)` if you really mean to",
                hit.name
            );
            self.report_secret(hit, message);
        }
    }

    /// The secret parts of `e`: everything made from a secret is one, up
    /// to `reveal(…)` (spec 8.7).
    pub(crate) fn secrets_in(&self, e: &Expr) -> Vec<Hit> {
        let mut hits = Vec::new();
        self.find_secrets(e, &mut hits);
        hits
    }

    /// Reports `hit` now when it is a secret already, or once every
    /// variable's secrecy is known when it is a variable (`secret_flow.rs`).
    pub(crate) fn report_secret(&mut self, hit: Hit, message: String) {
        match hit.var {
            None => self.error(hit.span, message),
            Some(def) => self.secret_flow.reaches.push((def, hit.span, message)),
        }
    }

    fn find_secrets(&self, e: &Expr, out: &mut Vec<Hit>) {
        let here = (e.span.start, e.span.end);
        let now = |name: String| Hit {
            span: e.span,
            name,
            var: None,
        };
        match &e.kind {
            ExprKind::Field { name, .. } if self.secret.contains(&here) => {
                out.push(now(name.name.clone()))
            }
            // A variable: secret if it is given one anywhere (`secret_flow.rs`).
            ExprKind::Name(id) => {
                if let Some(&def) = self.res.uses.get(&id.span.start)
                    && (self.locals.contains_key(&def) || self.secret_defs.contains(&def))
                {
                    out.push(Hit {
                        span: e.span,
                        name: id.name.clone(),
                        var: Some(def),
                    })
                }
            }
            // `env(…)` (spec 10.13), or a function whose result is
            // `secret` (8.7).
            ExprKind::Call { callee, .. } if self.secret.contains(&here) => {
                let name = match &callee.kind {
                    ExprKind::Name(id) => &id.name,
                    ExprKind::Field { name, .. } => &name.name,
                    _ => "the call",
                };
                out.push(now(format!("{name}(…)")))
            }
            ExprKind::Call { callee, args, .. } => {
                if matches!(&callee.kind, ExprKind::Name(id) if id.name == "reveal") {
                    return;
                }
                // A method's result is made from its receiver.
                if let ExprKind::Field { base, .. } = &callee.kind {
                    self.find_secrets(base, out);
                }
                // Arguments checked against parameters were refused there
                // when they hold a secret, or went to a `secret` one: the
                // result is secret only when the function's is.
                if !self.secret_flow.checked.contains(&here) {
                    for a in args {
                        self.find_secrets(&a.value, out);
                    }
                }
            }
            ExprKind::Field { base, .. }
            | ExprKind::TupleField { base, .. }
            | ExprKind::Unary(_, base)
            | ExprKind::FromEnd(base)
            | ExprKind::Try { expr: base, .. }
            | ExprKind::Spawn(_, base, _)
            | ExprKind::On { value: base, .. } => self.find_secrets(base, out),
            ExprKind::Binary(_, l, r)
            | ExprKind::Index { base: l, index: r }
            | ExprKind::Catch {
                expr: l,
                handler: r,
                ..
            } => {
                self.find_secrets(l, out);
                self.find_secrets(r, out);
            }
            ExprKind::Compare(first, links) => {
                self.find_secrets(first, out);
                links.iter().for_each(|(_, x)| self.find_secrets(x, out));
            }
            ExprKind::List(xs) | ExprKind::Tuple(xs) | ExprKind::Dims(xs) => {
                xs.iter().for_each(|x| self.find_secrets(x, out))
            }
            ExprKind::Map(pairs) => pairs.iter().for_each(|(k, v)| {
                self.find_secrets(k, out);
                self.find_secrets(v, out);
            }),
            ExprKind::AnonRecord(fields) => {
                fields.iter().for_each(|(_, x)| self.find_secrets(x, out))
            }
            ExprKind::With { base, fields } => {
                self.find_secrets(base, out);
                fields.iter().for_each(|(_, x)| self.find_secrets(x, out));
            }
            ExprKind::Part { start, end, step } => {
                for x in [start, end, step].into_iter().flatten() {
                    self.find_secrets(x, out);
                }
            }
            // A branch's value: not the condition (spec 8.7).
            ExprKind::If {
                then, otherwise, ..
            }
            | ExprKind::IfLet {
                then, otherwise, ..
            } => {
                if let Some(x) = tail(then) {
                    self.find_secrets(x, out);
                }
                if let Some(x) = otherwise {
                    self.find_secrets(x, out);
                }
            }
            ExprKind::Match { arms, .. } => arms
                .iter()
                .for_each(|arm| self.find_secrets(&arm.body, out)),
            ExprKind::Block(b)
            | ExprKind::Arena(b)
            | ExprKind::Unsafe(b)
            | ExprKind::Scoped { body: b, .. } => {
                if let Some(x) = tail(b) {
                    self.find_secrets(x, out);
                }
            }
            // A lambda can't return a secret but what its own parameters
            // were given (`lambdas.rs`); text with a secret in it is
            // refused where the secret goes in (`exprs.rs`).
            _ => {}
        }
    }

    /// A secret returned from a function, or a lambda, whose result isn't
    /// `secret` (spec 8.7): `name` is the function's. A lambda may give
    /// back what its own parameters were given: the call it is given to
    /// gives a secret then (`secret_flow.rs`).
    pub(crate) fn check_secret_return(&mut self, e: &Expr, name: &str) {
        let own = match name {
            "this lambda" => self.secret_flow.own.last().cloned().unwrap_or_default(),
            _ => Vec::new(),
        };
        for hit in self.secrets_in(e) {
            if hit.var.is_some_and(|d| own.contains(&d)) {
                continue;
            }
            let secret = &hit.name;
            let message = if name == "this lambda" {
                format!(
                    "`{secret}` is secret, and a lambda can't return a secret: return it from a named function with `-> secret …` (spec 8.7), or use `reveal(…)` if you really mean to"
                )
            } else {
                format!(
                    "`{secret}` is secret, so `{name}` can't return it: write `-> secret …` for its result (spec 8.7), or use `reveal(…)` if you really mean to"
                )
            };
            self.report_secret(hit, message);
        }
    }
}

/// A secret part of an expression: where, its name, and the variable it
/// is, whose secrecy is settled once the whole file is checked.
pub(crate) struct Hit {
    pub(crate) span: Span,
    pub(crate) name: String,
    pub(crate) var: Option<DefId>,
}

/// The last line of `b`, when it is a value.
fn tail(b: &Block) -> Option<&Expr> {
    match b.stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Expr(x)) => Some(x),
        _ => None,
    }
}
