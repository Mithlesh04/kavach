//! Rules of single operators beyond "both sides the same number type"
//! (spec 1.8): `**` and compound assignment.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;

impl Checker<'_> {
    /// `a ** b` works on whole numbers and floats, not on `Decimal`.
    pub(crate) fn check_power(&mut self, e: &Expr, t: &Ty) -> bool {
        if self.inf.resolve(t) != Ty::Prim(Prim::Decimal) {
            return true;
        }
        self.error(
            e.span,
            "`**` works on whole numbers and floats, not on `Decimal`: multiply instead",
        );
        false
    }

    /// `x op= v` is `x = x op v`, so `op` must apply to `x` (of type
    /// `t`), as it would in `x op v`. A map item (`ages[name] += 1`) is
    /// its value.
    pub(crate) fn check_compound(&mut self, op: BinaryOp, target: &Expr, t: &Ty) {
        let t = match self.inf.resolve(t) {
            Ty::Optional(inner) => *inner,
            other => other,
        };
        if op == BinaryOp::Add && matches!(t, Ty::Prim(Prim::String | Prim::Bytes)) {
            return;
        }
        let int_only = matches!(
            op,
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr
        );
        if self.require_number(target, &t, int_only) && op == BinaryOp::Pow {
            self.check_power(target, &t);
        }
    }
}

/// Whether `e` is a whole-number literal, or a float literal with no
/// fraction (`0.0`, `-1.0`): values `==` on floats compares safely.
fn whole_literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Float(text) => text
            .split_once('.')
            .is_some_and(|(_, frac)| frac.chars().all(|c| c == '0')),
        ExprKind::Unary(UnaryOp::Neg, x) => whole_literal(x),
        _ => false,
    }
}

/// `x % 2`, giving `x`.
fn halved(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        ExprKind::Binary(BinaryOp::Rem, x, two) if matches!(two.kind, ExprKind::Int(2)) => Some(x),
        _ => None,
    }
}

impl Checker<'_> {
    /// Comparisons that work but rarely mean what they say (spec 1.8):
    /// `==` on computed floats, and `x % 2 == 1` on signed numbers.
    pub(crate) fn check_compare(&mut self, op: BinaryOp, l: &Expr, r: &Expr, t: &Ty) {
        if !matches!(op, BinaryOp::Eq | BinaryOp::NotEq) || self.in_example {
            return;
        }
        // A literal's type not settled yet is the one it defaults to.
        let ty = match self.inf.resolve(t) {
            Ty::Var(v) if self.inf.kind(v) == crate::infer::VarKind::FloatLiteral => {
                Ty::Prim(Prim::Float)
            }
            Ty::Var(v) if self.inf.kind(v) == crate::infer::VarKind::IntLiteral => {
                Ty::Prim(Prim::Int)
            }
            t => t,
        };
        let exact = crate::constants::const_exact(l).is_some()
            && crate::constants::const_exact(r).is_some();
        if matches!(ty, Ty::Prim(Prim::Float | Prim::Float32))
            && !whole_literal(l)
            && !whole_literal(r)
            && !exact
        {
            let o = op.as_str();
            self.warn(
                l.span,
                &format!("`{o}` on floats compares every bit, and rounding makes most computed values differ (`0.1 + 0.2` worked out while running isn't `0.3`): use `a.near(b)`, or `Decimal` for exact amounts"),
            );
        }
        let signed = matches!(
            ty,
            Ty::Prim(Prim::Int | Prim::Int8 | Prim::Int16 | Prim::Int32 | Prim::BigInt)
        );
        // The side that isn't `x % 2`, if one is.
        let other = match (halved(l), halved(r)) {
            (Some(_), _) => Some(&r.kind),
            (_, Some(_)) => Some(&l.kind),
            _ => None,
        };
        if signed && matches!(other, Some(ExprKind::Int(1))) {
            let fix = if op == BinaryOp::Eq {
                "x % 2 != 0"
            } else {
                "x % 2 == 0"
            };
            let o = op.as_str();
            self.warn(
                l.span,
                &format!("`x % 2 {o} 1` is wrong for a negative odd `x`, since `-3 % 2` is `-1`: write `{fix}`"),
            );
        }
    }
}
