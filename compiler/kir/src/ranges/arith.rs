//! The range of `a op b`, and the checks and divisions it makes
//! unnecessary (`ranges.rs`).

use super::interval::{Range, limits};
use super::{Pass, Phase};
use crate::ir::*;

impl Pass {
    /// The range of `a op b`; in the last round, rewrites `e` when the
    /// ranges prove a check unnecessary.
    pub(super) fn binary(
        &mut self,
        e: &mut Expr,
        op: BinOp,
        ra: Option<Range>,
        rb: Option<Range>,
    ) -> Option<Range> {
        let ty = e.ty;
        if !ty.is_int() {
            return None;
        }
        let (Some(a), Some(b)) = (ra, rb) else {
            return Some(limits(ty));
        };
        let corners = |f: fn(i128, i128) -> i128| {
            let c = [f(a.lo, b.lo), f(a.lo, b.hi), f(a.hi, b.lo), f(a.hi, b.hi)];
            Range {
                lo: *c.iter().min().expect("four corners"),
                hi: *c.iter().max().expect("four corners"),
            }
        };
        let (r, fast) = match op {
            BinOp::Add => (corners(i128::saturating_add), FastOp::Add),
            BinOp::Sub => (corners(i128::saturating_sub), FastOp::Sub),
            BinOp::Mul => (corners(i128::saturating_mul), FastOp::Mul),
            BinOp::Rem if b.lo == b.hi && b.lo > 0 => {
                let c = b.lo;
                let r = if a.lo >= 0 {
                    Range {
                        lo: 0,
                        hi: a.hi.min(c - 1),
                    }
                } else {
                    Range {
                        lo: a.lo.max(-(c - 1)),
                        hi: a.hi.min(c - 1),
                    }
                };
                if self.phase == Phase::Rewrite && a.lo >= 0 {
                    rewrite_rem(e, a, c);
                }
                return Some(r);
            }
            BinOp::Div if b.lo == b.hi && b.lo > 0 => {
                if self.phase == Phase::Rewrite && a.lo >= 0 {
                    let small = a.hi.max(b.hi) <= SMALL;
                    to_fast(
                        e,
                        if small {
                            FastOp::DivSmall
                        } else {
                            FastOp::DivPositive
                        },
                    );
                }
                return Some(Range {
                    lo: a.lo / b.lo,
                    hi: a.hi / b.lo,
                });
            }
            BinOp::BitAnd if b.lo >= 0 && a.lo >= 0 => {
                return Some(Range {
                    lo: 0,
                    hi: a.hi.min(b.hi),
                });
            }
            // A mask that isn't negative clears the sign: `t & 1` is 0 or
            // 1 whatever `t` is.
            BinOp::BitAnd if b.lo >= 0 => return Some(Range { lo: 0, hi: b.hi }),
            _ => return Some(limits(ty)),
        };
        if self.phase == Phase::Rewrite && r.within(ty) {
            to_fast(e, fast);
        }
        Some(r.clamp(ty))
    }
}

/// The largest value 32-bit division takes.
const SMALL: i128 = u32::MAX as i128;

/// Turns the binary expression `e` into the proven operation `op`.
fn to_fast(e: &mut Expr, op: FastOp) {
    let ExprKind::Binary(_, x, y) = std::mem::replace(&mut e.kind, ExprKind::None) else {
        unreachable!("a binary expression");
    };
    e.kind = ExprKind::Fast(op, x, y);
}

/// `a % c` with `a >= 0`: `a` itself when it is always below `c`, one
/// subtract when it is below `2c`, otherwise an unsigned remainder.
fn rewrite_rem(e: &mut Expr, a: Range, c: i128) {
    if a.hi >= c {
        let op = if a.hi < 2 * c {
            FastOp::RemOnce
        } else if a.hi.max(c) <= SMALL {
            FastOp::RemSmall
        } else {
            FastOp::RemPositive
        };
        return to_fast(e, op);
    }
    if let ExprKind::Binary(_, x, _) = std::mem::replace(&mut e.kind, ExprKind::None) {
        e.kind = x.kind;
    }
}
