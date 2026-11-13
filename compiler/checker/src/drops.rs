//! `Drop` (spec 6.1): which types implement it, which values hold one of
//! them, and what making one needs.

use crate::checker::{Checker, Shape};
use crate::ty::Ty;
use kavach_ast::Span;
use kavach_resolver::DefId;
use std::collections::HashSet;

impl Checker<'_> {
    /// Whether type `def` implements `Drop`.
    pub(crate) fn implements_drop(&self, def: DefId) -> bool {
        self.drop_types.contains(&def)
    }

    /// Whether a value of type `t` is, or holds, a value of a type that
    /// implements `Drop`: then it lives until the end of its block.
    pub(crate) fn holds_drop(&self, t: &Ty) -> bool {
        self.holds(t, &mut HashSet::new())
    }

    fn holds(&self, t: &Ty, seen: &mut HashSet<DefId>) -> bool {
        match t {
            Ty::Adt(def, args) => {
                if self.implements_drop(*def) {
                    return true;
                }
                if args.iter().any(|a| self.holds(a, seen)) {
                    return true;
                }
                if !seen.insert(*def) {
                    return false;
                }
                match self.types.get(def).map(|t| &t.shape) {
                    Some(Shape::Record(fields)) => fields.iter().any(|f| self.holds(&f.ty, seen)),
                    Some(Shape::Union(variants)) => variants
                        .iter()
                        .any(|v| v.fields.iter().any(|f| self.holds(&f.ty, seen))),
                    _ => false,
                }
            }
            Ty::List(t) | Ty::Optional(t) | Ty::Set(t) => self.holds(t, seen),
            Ty::Map(k, v) => self.holds(k, seen) || self.holds(v, seen),
            Ty::Tuple(ts) => ts.iter().any(|t| self.holds(t, seen)),
            Ty::Anon(fields) => fields.iter().any(|(_, t)| self.holds(t, seen)),
            _ => false,
        }
    }

    /// Making a value of record type `def` counts as calling its `drop`:
    /// what `drop` needs (`uses`) is needed where the value is made.
    pub(crate) fn note_drop(&mut self, def: DefId, span: Span) {
        if !self.implements_drop(def) {
            return;
        }
        let drop = self.methods.get(&def).and_then(|m| m.get("drop"));
        if let Some(d) = drop.and_then(|sig| sig.def) {
            self.note_fn(d, span);
        }
    }

    /// `const _ = Guard(…)`: `_` names nothing, so a `Drop` value given to
    /// it drops at the end of the statement (spec 6.1), the trap of Rust's
    /// `let _ = lock()`.
    pub(crate) fn warn_unnamed_drop(&mut self, pattern: &kavach_ast::Pattern, ty: &Ty) {
        let ty = self.inf.resolve(ty);
        if matches!(pattern.kind, kavach_ast::PatternKind::Wildcard) && self.holds_drop(&ty) {
            self.warn(
                pattern.span,
                "`_` names nothing, so this value drops here, at the end of the statement: name it `_guard` to keep it to the end of the block (spec 6.1)",
            );
        }
    }
}
