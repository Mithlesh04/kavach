//! Ranges of integer values (`lo..=hi`), and what a branch condition
//! says about a local's range.

use crate::ir::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Range {
    pub(super) lo: i128,
    pub(super) hi: i128,
}

/// The smallest and largest value of an integer type.
pub(super) fn limits(t: Type) -> Range {
    let bits = t.bits();
    if t.is_signed() {
        Range {
            lo: -(1i128 << (bits - 1)),
            hi: (1i128 << (bits - 1)) - 1,
        }
    } else {
        Range {
            lo: 0,
            hi: (1i128 << bits) - 1,
        }
    }
}

impl Range {
    pub(super) fn join(self, o: Range) -> Range {
        Range {
            lo: self.lo.min(o.lo),
            hi: self.hi.max(o.hi),
        }
    }

    pub(super) fn within(self, t: Type) -> bool {
        let l = limits(t);
        self.lo >= l.lo && self.hi <= l.hi
    }

    /// Only the values that fit `t`: an operation that would give any
    /// other value panics instead.
    pub(super) fn clamp(self, t: Type) -> Range {
        let l = limits(t);
        Range {
            lo: self.lo.clamp(l.lo, l.hi),
            hi: self.hi.clamp(l.lo, l.hi),
        }
    }

    /// The values in both ranges (`None` when there are none).
    pub(super) fn meet(self, o: Range) -> Option<Range> {
        let r = Range {
            lo: self.lo.max(o.lo),
            hi: self.hi.min(o.hi),
        };
        (r.lo <= r.hi).then_some(r)
    }
}

/// What a condition says about locals: their ranges where it holds, and
/// where it doesn't.
pub(super) type Facts = (Vec<(Local, Range)>, Vec<(Local, Range)>);

/// What `cond` says about locals in each branch of an `if`: for `x < c`,
/// `x` is below `c` in the `then` branch and at least `c` in the `else`
/// branch. Comparisons of an integer local with a constant, and `&&`s of
/// them: where `a && b` holds, both do (as in `version.rs`'s test
/// `0 <= x && x <= hi && total >= 0`); where it doesn't, nothing is known.
pub(super) fn branch_facts(cond: &Expr) -> Facts {
    if let ExprKind::And(a, b) = &cond.kind {
        let (mut yes, _) = branch_facts(a);
        yes.extend(branch_facts(b).0);
        return (yes, Vec::new());
    }
    compare_facts(cond).map_or_else(Default::default, |(l, yes, no)| {
        (vec![(l, yes)], vec![(l, no)])
    })
}

/// What a comparison of an integer local with a constant says about it.
fn compare_facts(cond: &Expr) -> Option<(Local, Range, Range)> {
    let ExprKind::Binary(op, a, b) = &cond.kind else {
        return None;
    };
    // `c < x` is `x > c`.
    let (l, c, op) = match (&a.kind, &b.kind) {
        (ExprKind::Local(l), ExprKind::Int(c)) if a.ty.is_int() => (*l, *c, *op),
        (ExprKind::Int(c), ExprKind::Local(l)) if b.ty.is_int() => {
            let flipped = match op {
                BinOp::Lt => BinOp::Gt,
                BinOp::Le => BinOp::Ge,
                BinOp::Gt => BinOp::Lt,
                BinOp::Ge => BinOp::Le,
                other => *other,
            };
            (*l, *c, flipped)
        }
        _ => return None,
    };
    let below = |hi: i128| Range { lo: i128::MIN, hi };
    let above = |lo: i128| Range { lo, hi: i128::MAX };
    let (yes, no) = match op {
        BinOp::Lt => (below(c - 1), above(c)),
        BinOp::Le => (below(c), above(c + 1)),
        BinOp::Gt => (above(c + 1), below(c)),
        BinOp::Ge => (above(c), below(c - 1)),
        _ => return None,
    };
    Some((l, yes, no))
}
