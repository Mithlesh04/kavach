//! Secrets (spec 2.11, 8.7): where a secret value may go, and where it
//! may not: text, prints, parameters and results that aren't `secret`.

use crate::checker::Checker;
use kavach_ast::*;

impl Checker<'_> {
    /// Whether `e` reads a secret field, directly or inside a string (spec 2.11).
    pub(crate) fn check_secret(&mut self, e: &Expr, into: &str) {
        let mut hits = Vec::new();
        self.find_secrets(e, &mut hits);
        for (span, name) in hits {
            let into = if into == "text" {
                "text".to_string()
            } else {
                format!("`{into}`")
            };
            self.error(span, format!("`{name}` is secret, so it can't go into {into}; use `reveal(…)` if you really mean to"));
        }
    }

    /// Whether `e`'s value is a secret: a secret field, a secret parameter,
    /// `env(…)`, or a variable set from one (spec 8.7, 10.13).
    pub(crate) fn is_secret(&self, e: &Expr) -> bool {
        let mut hits = Vec::new();
        self.find_secrets(e, &mut hits);
        !hits.is_empty()
    }

    /// Passing a secret where nothing may receive it: an argument of a
    /// parameter, or a field, that isn't marked `secret` (spec 8.7).
    pub(crate) fn check_secret_arg(&mut self, e: &Expr, what: &str, name: &str, owner: &str) {
        let mut hits = Vec::new();
        self.find_secrets(e, &mut hits);
        for (span, secret) in hits {
            self.error(
                span,
                format!("`{secret}` is secret, so it can't go into the {what} `{name}` of {owner}, which isn't `secret`; mark it `secret`, or use `reveal(…)` if you really mean to"),
            );
        }
    }

    fn find_secrets(&self, e: &Expr, out: &mut Vec<(Span, String)>) {
        match &e.kind {
            ExprKind::Field { name, .. } if self.secret.contains(&(e.span.start, e.span.end)) => {
                out.push((e.span, name.name.clone()))
            }
            ExprKind::Name(id)
                if self
                    .res
                    .uses
                    .get(&id.span.start)
                    .is_some_and(|d| self.secret_defs.contains(d)) =>
            {
                out.push((e.span, id.name.clone()))
            }
            // `env(…)` (spec 10.13), or a function whose result is
            // `secret` (8.7).
            ExprKind::Call { callee, .. } if self.secret.contains(&(e.span.start, e.span.end)) => {
                let name = match &callee.kind {
                    ExprKind::Name(id) => &id.name,
                    ExprKind::Field { name, .. } => &name.name,
                    _ => "the call",
                };
                out.push((e.span, format!("{name}(…)")))
            }
            ExprKind::Binary(_, l, r) => {
                self.find_secrets(l, out);
                self.find_secrets(r, out);
            }
            ExprKind::Try { expr, .. } => self.find_secrets(expr, out),
            ExprKind::Catch { expr, handler, .. } => {
                self.find_secrets(expr, out);
                self.find_secrets(handler, out);
            }
            _ => {}
        }
    }

    /// A secret returned from a function, or a lambda, whose result isn't
    /// `secret` (spec 8.7): `name` is the function's.
    pub(crate) fn check_secret_return(&mut self, e: &Expr, name: &str) {
        let mut hits = Vec::new();
        self.returned_secrets(e, &mut hits);
        for (span, secret) in hits {
            let message = if name == "this lambda" {
                format!(
                    "`{secret}` is secret, and a lambda can't return a secret: return it from a named function with `-> secret …` (spec 8.7), or use `reveal(…)` if you really mean to"
                )
            } else {
                format!(
                    "`{secret}` is secret, so `{name}` can't return it: write `-> secret …` for its result (spec 8.7), or use `reveal(…)` if you really mean to"
                )
            };
            self.error(span, message);
        }
    }

    /// The secrets a value gives: through the last line of a block, and
    /// each branch of an `if` or `match`.
    fn returned_secrets(&self, e: &Expr, out: &mut Vec<(Span, String)>) {
        match &e.kind {
            ExprKind::Block(b) => {
                if let Some(x) = tail(b) {
                    self.returned_secrets(x, out);
                }
            }
            ExprKind::If {
                then, otherwise, ..
            }
            | ExprKind::IfLet {
                then, otherwise, ..
            } => {
                if let Some(x) = tail(then) {
                    self.returned_secrets(x, out);
                }
                if let Some(x) = otherwise {
                    self.returned_secrets(x, out);
                }
            }
            ExprKind::Match { arms, .. } => {
                for arm in arms {
                    self.returned_secrets(&arm.body, out);
                }
            }
            _ => self.find_secrets(e, out),
        }
    }
}

/// The last line of `b`, when it is a value.
fn tail(b: &Block) -> Option<&Expr> {
    match b.stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Expr(x)) => Some(x),
        _ => None,
    }
}
