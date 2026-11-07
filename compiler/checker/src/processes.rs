//! `spawn process f(args)` (spec 9.8, decision 0075): the call runs in a
//! process of its own, so what goes in and comes out is copied, as JSON.
//! It must be a call of a function by its name, not generic, and its
//! arguments, result and error must be values JSON carries.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::{Expr, ExprKind, Span};
use kavach_resolver::DefKind;

/// The built-in error types, which carry their message across.
const ERRORS: [&str; 4] = ["Error", "ParseError", "CliError", "RegexError"];

impl Checker<'_> {
    /// Checks `x` of `spawn process x`, a call whose type is `t`.
    pub(crate) fn check_process_call(&mut self, x: &Expr, t: &Ty) {
        let ExprKind::Call { callee, args, .. } = &x.kind else {
            self.error(
                x.span,
                "`spawn process` takes a call of a function, not a block: put the block in a function and call it (spec 9.8)",
            );
            return;
        };
        let named = match &callee.kind {
            ExprKind::Name(id) => (self.res.uses.get(&id.span.start).copied())
                .filter(|d| self.res.defs[*d].kind == DefKind::Fn),
            _ => None,
        };
        let Some(def) = named else {
            self.error(
                callee.span,
                "`spawn process` takes a call of a function by its name, such as `spawn process convert(file)` (spec 9.8)",
            );
            return;
        };
        if self.fns.get(&def).is_some_and(|s| !s.generics.is_empty()) {
            self.error(
                callee.span,
                "`spawn process` can't call a generic function yet",
            );
            return;
        }
        for a in args {
            let why = self
                .type_at(a.value.span)
                .and_then(|t| self.not_encodable(&t));
            if let Some(why) = why {
                self.error(
                    a.value.span,
                    format!("{why}, so it can't go to another process (spec 9.8)"),
                );
            }
        }
        let (ok, err) = match self.inf.resolve(t) {
            Ty::Result(ok, err) => (*ok, Some(*err)),
            t => (t, None),
        };
        self.comes_back(&ok, x.span, "the call's result");
        if let Some(err) = err {
            self.comes_back(&err, x.span, "the call's error");
        }
    }

    /// Reports `what`, of type `t`, if it can't come back from another
    /// process: nothing, a built-in error and decodable values can.
    fn comes_back(&mut self, t: &Ty, span: Span, what: &str) {
        let t = self.inf.resolve(t);
        if matches!(t, Ty::Unit | Ty::Never | Ty::Error)
            || matches!(t, Ty::Builtin(name, _) if ERRORS.contains(&name))
        {
            return;
        }
        if let Some(why) = self.not_decodable(&t) {
            self.error(
                span,
                format!("{why}, so {what} can't come back from another process (spec 9.8)"),
            );
        }
    }

    /// The type the checker gave the expression at `span`.
    fn type_at(&self, span: Span) -> Option<Ty> {
        (self.expr_types.iter().rev())
            .find(|(s, _)| *s == span)
            .map(|(_, t)| t.clone())
    }
}
