//! Which values have an order (spec 1.8), for `<`, `sort`, `min`, `max`
//! and `sort_by`'s key; and the waits refused inside `Shared.update`
//! (spec 6.2).

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::Span;

fn number(t: &Ty) -> bool {
    matches!(t, Ty::Prim(p) if p.is_int() || matches!(p, Prim::Float | Prim::Float32 | Prim::Decimal | Prim::BigInt))
}

/// Values with an order (spec 1.8), for `<`, `sort`, `min`, `max` and
/// the key of `sort_by`: numbers, text, characters, `Bool` (`false`
/// first) and tuples of them, item by item.
fn orderable(t: &Ty) -> bool {
    number(t)
        || matches!(t, Ty::Prim(Prim::String | Prim::Char | Prim::Bool))
        || matches!(t, Ty::Tuple(items) if items.iter().all(orderable))
}

impl Checker<'_> {
    /// `what` waits: an error inside the function given to `update`,
    /// which runs with the `Shared` value locked (spec 6.2, decision 0052).
    pub(crate) fn no_wait_in_update(&mut self, span: Span, what: &str) {
        if self.updating > 0 {
            self.error(
                span,
                format!("{what} waits, and the function given to `update` can't wait: the `Shared` value stays locked until it returns. Get what the update needs first, then update (spec 6.2)"),
            );
        }
    }

    /// Whether values of `t` have an order; `None` while `t` isn't known.
    pub(crate) fn ordered(&self, t: &Ty) -> Option<bool> {
        let t = self.inf.resolve(t);
        let unknown = |t: &Ty| matches!(t, Ty::Var(_) | Ty::Error);
        let open = match &t {
            Ty::Tuple(items) => items.iter().any(unknown),
            other => unknown(other),
        };
        (!open).then(|| orderable(&t))
    }

    /// Reports a value without an order where one is needed.
    pub(crate) fn require_order(&mut self, t: &Ty, span: Span, what: &str) {
        if self.ordered(t) == Some(false) {
            let shown = self.show(t);
            self.error(
                span,
                format!("{what} needs values with an order (numbers, text, characters, `Bool`, or tuples of them), and `{shown}` has none"),
            );
        }
    }
}
