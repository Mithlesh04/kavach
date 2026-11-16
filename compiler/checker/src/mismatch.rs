//! Messages about a value that doesn't fit where it is used, and the
//! small helpers they share (split from `checker.rs`).

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::{Expr, ExprKind};

impl Checker<'_> {
    pub(crate) fn mismatch(&mut self, e: &Expr, expected: &Ty, actual: &Ty) {
        let (want, got) = (self.show(expected), self.show(actual));
        let (we, ga) = (self.inf.resolve(expected), self.inf.resolve(actual));
        let message = match (&we, &ga, &e.kind) {
            (_, _, ExprKind::None) => format!(
                "`{want}` can't hold `none`: write `{want}?` for a value that may be missing"
            ),
            (Ty::Prim(p), Ty::Prim(q), _) if p.is_numeric() && q.is_numeric() => format!(
                "there are no implicit number conversions: expected `{want}`, found `{got}`; convert with `{want}(…)`"
            ),
            (_, Ty::Unit, _) => format!("expected `{want}`, but this returns nothing"),
            (Ty::Fn(_, w, _), Ty::Fn(_, g, _), _)
                if matches!(**g, Ty::Result(..)) && !matches!(**w, Ty::Result(..)) =>
            {
                format!(
                    "`{got}` can fail, so it doesn't fit `{want}`: handle the error inside it, or write `! ErrorType` in the function type (spec 3.4)"
                )
            }
            (Ty::Builtin(a, x), Ty::Builtin(b, y), _)
                if *a == "Shared"
                    && *b == "Shared"
                    && matches!(x.first(), Some(Ty::Optional(t)) if Some(&**t) == y.first()) =>
            {
                format!(
                    "expected `{want}`, found `{got}`: every copy of a `Shared` sees one value, so its type can't change; make it as `{want}` from the start"
                )
            }
            _ => format!("expected `{want}`, found `{got}`"),
        };
        self.error(e.span, message);
    }

    /// Whether `ty` carries no value worth keeping.
    pub(crate) fn is_nothing(&self, ty: &Ty) -> bool {
        matches!(
            self.inf.shallow(ty),
            Ty::Unit | Ty::Never | Ty::Error | Ty::Var(_)
        )
    }

    /// `the call to add`-style description of an expression, for messages.
    pub(crate) fn describe(&self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Call { callee, .. } => match &callee.kind {
                ExprKind::Name(id) => format!("`{}`", id.name),
                ExprKind::Field { name, .. } => format!("`{}`", name.name),
                _ => "this call".to_string(),
            },
            _ => "this".to_string(),
        }
    }
}
