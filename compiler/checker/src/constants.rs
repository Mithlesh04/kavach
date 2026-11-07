//! Constants of whole-number literals (spec 2.1, decision 0041), worked
//! out exactly: one that doesn't fit `Int` is a `BigInt` when no type is
//! written, and one that doesn't fit the written type is an error.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use num_bigint::{BigInt, Sign};

/// The most bits a constant power may have, so `2 ** 2 ** 40` doesn't
/// fill the compiler's memory: such a power is left to run time.
const MAX_BITS: u64 = 1 << 20;

/// The exact value of `e` when it is whole-number literals joined by
/// `+ - * / % **` and a minus; `None` otherwise, or when it can't be
/// worked out (a division by zero, a negative or huge exponent).
pub(crate) fn const_int(e: &Expr) -> Option<BigInt> {
    match &e.kind {
        ExprKind::Int(n) => Some(BigInt::from(*n)),
        ExprKind::BigInt(digits) => digits.parse().ok(),
        ExprKind::Unary(UnaryOp::Neg, x) => Some(-const_int(x)?),
        ExprKind::Binary(op, l, r) => {
            let (a, b) = (const_int(l)?, const_int(r)?);
            let zero = b.sign() == Sign::NoSign;
            Some(match op {
                BinaryOp::Add => a + b,
                BinaryOp::Sub => a - b,
                BinaryOp::Mul => a * b,
                BinaryOp::Div | BinaryOp::Rem if zero => return None,
                BinaryOp::Div => a / b,
                BinaryOp::Rem => a % b,
                BinaryOp::Pow => {
                    let exp = u32::try_from(&b).ok()?;
                    if a.bits().saturating_mul(u64::from(exp)) > MAX_BITS {
                        return None;
                    }
                    a.pow(exp)
                }
                _ => return None,
            })
        }
        _ => None,
    }
}

/// The smallest and largest value of whole-number type `p`.
fn range(p: Prim) -> (i128, i128) {
    match p {
        Prim::Int8 => (i8::MIN.into(), i8::MAX.into()),
        Prim::Int16 => (i16::MIN.into(), i16::MAX.into()),
        Prim::Int32 => (i32::MIN.into(), i32::MAX.into()),
        Prim::UInt8 => (0, u8::MAX.into()),
        Prim::UInt16 => (0, u16::MAX.into()),
        Prim::UInt32 => (0, u32::MAX.into()),
        Prim::UInt64 => (0, u64::MAX.into()),
        _ => (i64::MIN.into(), i64::MAX.into()),
    }
}

/// Whether `v` fits whole-number type `p`.
pub(crate) fn fits(v: &BigInt, p: Prim) -> bool {
    let (lo, hi) = range(p);
    *v >= BigInt::from(lo) && *v <= BigInt::from(hi)
}

/// What to write instead when `v` doesn't fit `p`: the smallest bigger
/// type of the same kind (signed or not) that holds it, else `BigInt`.
fn suggestion(v: &BigInt, p: Prim) -> String {
    let signed = [Prim::Int8, Prim::Int16, Prim::Int32, Prim::Int];
    let unsigned = [Prim::UInt8, Prim::UInt16, Prim::UInt32, Prim::UInt64];
    let order = if unsigned.contains(&p) && v.sign() != Sign::Minus {
        &unsigned
    } else {
        &signed
    };
    match order.iter().find(|q| fits(v, **q)) {
        Some(q) => format!("use `{}`", q.name()),
        None => "use `BigInt`, which holds a whole number of any size".to_string(),
    }
}

impl Checker<'_> {
    /// `-n`, a literal with a minus, whose value fits `p`: one literal of
    /// type `p`, so `-128` is an `Int8` though `128` alone isn't. `None`
    /// for anything else, which the usual rules type.
    fn negative_literal(&mut self, e: &Expr, p: Prim) -> Option<Ty> {
        let ExprKind::Unary(UnaryOp::Neg, inner) = &e.kind else {
            return None;
        };
        let ExprKind::Int(n) = inner.kind else {
            return None;
        };
        if n <= i64::MAX as u64 && !matches!(p, Prim::Int8 | Prim::Int16 | Prim::Int32) {
            return None;
        }
        self.literals.push((inner.span.start, Ty::Prim(p)));
        Some(Ty::Prim(p))
    }

    /// The type of `e` when it is a constant of whole-number literals the
    /// usual rules can't type: a `BigInt`, or an error for a value that
    /// doesn't fit the type `expected`. `None` leaves `e` to them.
    pub(crate) fn big_constant(&mut self, e: &Expr, expected: Option<&Ty>) -> Option<Ty> {
        let candidate = matches!(
            e.kind,
            ExprKind::Int(_)
                | ExprKind::BigInt(_)
                | ExprKind::Binary(..)
                | ExprKind::Unary(UnaryOp::Neg, _)
        );
        if !candidate {
            return None;
        }
        let v = const_int(e)?;
        match expected.map(|t| self.inf.resolve(t)) {
            Some(Ty::Prim(Prim::BigInt)) => {}
            Some(Ty::Prim(p)) if p.is_int() => {
                if fits(&v, p) {
                    return self.negative_literal(e, p);
                }
                let (name, (lo, hi)) = (p.name(), range(p));
                let fix = suggestion(&v, p);
                self.error(
                    e.span,
                    format!("`{v}` doesn't fit in `{name}`, which holds {lo} to {hi}: {fix}"),
                );
                return Some(Ty::Prim(p));
            }
            // A long literal where a `Decimal` or a float is expected is
            // one of those.
            Some(Ty::Prim(p)) if matches!(e.kind, ExprKind::BigInt(_)) => {
                self.literals.push((e.span.start, Ty::Prim(p)));
                return Some(Ty::Prim(p));
            }
            Some(Ty::Prim(_)) => return None,
            // `-9223372036854775808`: the smallest `Int`, whose digits alone
            // don't fit.
            _ if fits(&v, Prim::Int) => return self.negative_literal(e, Prim::Int),
            _ => {}
        }
        self.big_constants
            .insert((e.span.start, e.span.end), v.to_string());
        Some(Ty::Prim(Prim::BigInt))
    }
}

/// A number as an exact fraction `num / den`, `den` above 0.
#[derive(Clone)]
pub(crate) struct Exact {
    num: BigInt,
    den: BigInt,
}

impl Exact {
    fn whole(n: BigInt) -> Exact {
        Exact {
            num: n,
            den: BigInt::from(1),
        }
    }

    /// A float literal's text (`2.5`, `1_000.0`, `2.5e-3`), exactly.
    fn of_text(text: &str) -> Option<Exact> {
        let text = text.replace('_', "");
        let (number, exponent) = match text.split_once(['e', 'E']) {
            Some((n, e)) => (n.to_string(), e.parse::<i32>().ok()?),
            None => (text, 0),
        };
        let (int, frac) = number.split_once('.').unwrap_or((&number, ""));
        let digits: BigInt = format!("{int}{frac}").parse().ok()?;
        let shift = exponent - frac.len() as i32;
        let ten = BigInt::from(10).pow(shift.unsigned_abs());
        Some(if shift >= 0 {
            Exact::whole(digits * ten)
        } else {
            Exact {
                num: digits,
                den: ten,
            }
        })
    }

    fn combine(a: Exact, op: BinaryOp, b: Exact) -> Option<Exact> {
        let (num, den) = match op {
            BinaryOp::Add => (&a.num * &b.den + &b.num * &a.den, &a.den * &b.den),
            BinaryOp::Sub => (&a.num * &b.den - &b.num * &a.den, &a.den * &b.den),
            BinaryOp::Mul => (&a.num * &b.num, &a.den * &b.den),
            BinaryOp::Div if b.num.sign() == Sign::NoSign => return None,
            BinaryOp::Div => (&a.num * &b.den, &a.den * &b.num),
            BinaryOp::Pow if b.den == BigInt::from(1) => {
                let e = u32::try_from(&b.num).ok()?;
                if a.num.bits().max(a.den.bits()).saturating_mul(u64::from(e)) > MAX_BITS {
                    return None;
                }
                (a.num.pow(e), a.den.pow(e))
            }
            _ => return None,
        };
        let (num, den) = if den.sign() == Sign::Minus {
            (-num, -den)
        } else {
            (num, den)
        };
        Some(Exact { num, den })
    }

    /// The `Float` nearest this value: its first 40 digits and one more
    /// that says whether anything follows, read by Rust's correctly
    /// rounded parser.
    pub(crate) fn to_f64(&self) -> f64 {
        let digits = |n: &BigInt| n.magnitude().to_string().len() as i64;
        let k = 41 + digits(&self.den) - digits(&self.num);
        let (scaled, den) = if k >= 0 {
            (
                self.num.magnitude() * BigInt::from(10).pow(k as u32).magnitude(),
                self.den.magnitude().clone(),
            )
        } else {
            (
                self.num.magnitude().clone(),
                self.den.magnitude() * BigInt::from(10).pow((-k) as u32).magnitude(),
            )
        };
        let (q, r) = (&scaled / &den, &scaled % &den);
        let sticky = u8::from(r != num_bigint::BigUint::default());
        let sign = if self.num.sign() == Sign::Minus {
            "-"
        } else {
            ""
        };
        format!("{sign}{q}{sticky}e{}", -k - 1)
            .parse()
            .unwrap_or(f64::NAN)
    }
}

/// `e` exactly, when it is number literals (whole or not) joined by
/// `+ - * /`, a minus, and `**` with a whole exponent.
pub(crate) fn const_exact(e: &Expr) -> Option<Exact> {
    match &e.kind {
        ExprKind::Int(n) => Some(Exact::whole(BigInt::from(*n))),
        ExprKind::BigInt(digits) => Some(Exact::whole(digits.parse().ok()?)),
        ExprKind::Float(text) => Exact::of_text(text),
        ExprKind::Unary(UnaryOp::Neg, x) => {
            let v = const_exact(x)?;
            Some(Exact { num: -v.num, ..v })
        }
        ExprKind::Binary(op, l, r) => Exact::combine(const_exact(l)?, *op, const_exact(r)?),
        _ => None,
    }
}

impl Checker<'_> {
    /// Notes `e` when it is a constant with an operator: if its type turns
    /// out to be a float, it is rounded once from its exact value
    /// (decision 0043).
    pub(crate) fn note_exact_constant(&mut self, e: &Expr) {
        if !matches!(
            e.kind,
            ExprKind::Binary(..) | ExprKind::Unary(UnaryOp::Neg, _)
        ) {
            return;
        }
        if let Some(v) = const_exact(e) {
            self.exact_constants.push(((e.span.start, e.span.end), v));
        }
    }
}
