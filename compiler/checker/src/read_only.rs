//! `pub(read)` fields (spec 10.1): other modules read them, but only the
//! module that declares the type changes them, so it keeps the type's
//! invariants. A change elsewhere is a compile error that says so.

use crate::checker::Checker;
use kavach_ast::*;

impl Checker<'_> {
    /// `target` changes (`how`): an error when a `pub(read)` field on its
    /// path was named outside its type's module.
    pub(crate) fn check_read_only_target(&mut self, target: &Expr, how: &str) {
        let mut at = target;
        loop {
            match &at.kind {
                ExprKind::Field { base, name, .. } => {
                    self.check_read_only_name(name, how);
                    at = base;
                }
                ExprKind::Index { base, .. } | ExprKind::TupleField { base, .. } => at = base,
                _ => return,
            }
        }
    }

    /// `name`, a field named outside its type's module, changes (`how`).
    pub(crate) fn check_read_only_name(&mut self, name: &Ident, how: &str) {
        if self.read_only_uses.contains(&name.span.start) {
            self.error(
                name.span,
                format!("`{}` is `pub(read)`: other modules read it, but only the module that declares its type changes it, so it can't be {how} here (spec 10.1)", name.name),
            );
        }
    }
}
