// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Whole numbers with numbers in their types (spec 2.1, 2.9, decision
//! 0137): `Int<bits: 24>`, `UInt<bits: 12>` and `Int<range: 1..=65535>`.
//! A width of 8, 16, 32 or 64 is `Int8` … `UInt64`, so those stay one
//! type with two spellings.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// The widest `bits` built so far (spec 2.1): wider ones, up to 65,535,
/// are a compile error that says so.
pub(crate) const BUILT_BITS: i64 = 64;

/// The names made so far, by width or range ends and sign.
type Names = Mutex<HashMap<(i64, i64, bool), &'static str>>;

/// The name of an `Int<bits: n>`, `UInt<bits: n>` or `Int<range: …>`,
/// made once for each type and kept for the rest of the run.
pub(crate) fn name(p: Prim) -> &'static str {
    static NAMES: OnceLock<Names> = OnceLock::new();
    let (key, text) = match p {
        Prim::Bits(n, signed) => (
            (i64::from(n), 0, signed),
            format!("{}Int<bits: {n}>", if signed { "" } else { "U" }),
        ),
        Prim::Ranged(lo, hi) => ((lo, hi, true), format!("Int<range: {lo}..={hi}>")),
        _ => return "?",
    };
    let names = NAMES.get_or_init(Default::default);
    let mut names = names.lock().unwrap_or_else(|e| e.into_inner());
    names
        .entry(key)
        .or_insert_with(|| Box::leak(text.into_boxed_str()))
}

/// The type of `n` bits, signed or not: `Int8` … `UInt64` for those
/// widths, else a `Bits`.
pub fn bits_type(n: u16, signed: bool) -> Prim {
    match (n, signed) {
        (8, true) => Prim::Int8,
        (16, true) => Prim::Int16,
        (32, true) => Prim::Int32,
        (64, true) => Prim::Int,
        (8, false) => Prim::UInt8,
        (16, false) => Prim::UInt16,
        (32, false) => Prim::UInt32,
        (64, false) => Prim::UInt64,
        _ => Prim::Bits(n, signed),
    }
}

/// Why a range has no `wrapping_*` methods (spec 2.1).
pub(crate) fn range_hint(recv: &Ty, method: &str) -> Option<String> {
    match recv {
        Ty::Prim(Prim::Ranged(..)) if method.starts_with("wrapping_") => Some(
            "wrapping has no meaning for a range: use `checked_add` and the others, which give `none` outside it".to_string(),
        ),
        _ => None,
    }
}

impl Checker<'_> {
    /// `Int<bits: n>`, `UInt<bits: n>` or `Int<range: a..=b>`, from the
    /// numbers written after `what` (`Int` or `UInt`).
    pub(crate) fn int_type(&mut self, what: &str, nums: &[&TypeExpr], span: Span) -> Ty {
        let fallback = Ty::Prim(if what == "UInt" {
            Prim::UInt64
        } else {
            Prim::Int
        });
        let named: Vec<(&Ident, &Expr, Span)> = (nums.iter())
            .filter_map(|n| match &n.kind {
                TypeKind::Num {
                    name: Some(name),
                    value,
                } => Some((name, &**value, n.span)),
                TypeKind::Num { name: None, value } => {
                    let v = self
                        .const_num(value)
                        .map_or("24".to_string(), |v| v.to_string());
                    self.error(
                        n.span,
                        format!(
                            "a number in a type has a name: write `{what}<bits: {v}>` (spec 2.9)"
                        ),
                    );
                    None
                }
                _ => None,
            })
            .collect();
        if named.len() > 1 {
            self.error(
                span,
                format!("`{what}` takes one number: `bits` or `range` (spec 2.1)"),
            );
            return fallback;
        }
        let Some((name, value, at)) = named.first().copied() else {
            return fallback;
        };
        let made = match (name.name.as_str(), what) {
            ("bits", _) => self.bits_of(what, value, at),
            ("range", "Int") => self.range_of(value),
            ("range", _) => {
                self.error(
                    name.span,
                    "a range is written on `Int`: `Int<range: 0..=4095>` (spec 2.1)",
                );
                None
            }
            (other, _) => {
                self.error(
                    name.span,
                    format!(
                        "`{what}` has no number `{other}`: it takes `bits` or `range` (spec 2.1)"
                    ),
                );
                None
            }
        };
        match made {
            Some(p) => {
                if p.is_narrow() {
                    self.sizes.wholes.insert((span.start, span.end), p);
                }
                Ty::Prim(p)
            }
            None => fallback,
        }
    }

    /// `bits: n`: from 1 to 65,535 (spec 2.1).
    fn bits_of(&mut self, what: &str, value: &Expr, at: Span) -> Option<Prim> {
        let Some(n) = self.const_num(value) else {
            self.error(
                value.span,
                "a width is a whole-number literal or a constant known while compiling (spec 2.9)",
            );
            return None;
        };
        if !(1..=65_535).contains(&n) {
            self.error(
                value.span,
                format!("a width is from 1 to 65,535 bits, not {n} (spec 2.1)"),
            );
            return None;
        }
        if n > BUILT_BITS {
            self.error(
                at,
                format!("`{what}<bits: {n}>`: widths over 64 bits are not built yet; `BigInt` holds a whole number of any size (spec 2.1)"),
            );
            return None;
        }
        Some(bits_type(n as u16, what == "Int"))
    }

    /// `range: a..=b` or `range: a..b`, both ends known while compiling.
    fn range_of(&mut self, value: &Expr) -> Option<Prim> {
        let ExprKind::Binary(op @ (BinaryOp::Range | BinaryOp::RangeInclusive), l, r) = &value.kind
        else {
            self.error(
                value.span,
                "a range in a type has both its ends: `Int<range: 1..=65535>` (spec 2.1)",
            );
            return None;
        };
        let (Some(lo), Some(end)) = (self.const_num(l), self.const_num(r)) else {
            self.error(
                value.span,
                "a range's ends are whole-number literals or constants known while compiling (spec 2.1)",
            );
            return None;
        };
        let hi = match op {
            BinaryOp::Range => end.checked_sub(1),
            _ => Some(end),
        };
        match hi {
            Some(hi) if lo <= hi => Some(Prim::Ranged(lo, hi)),
            _ => {
                let dots = if *op == BinaryOp::Range { ".." } else { "..=" };
                let text = format!("{lo}{dots}{end}");
                self.error(
                    value.span,
                    format!("the range `{text}` is empty: a value could never be in it (spec 2.1)"),
                );
                None
            }
        }
    }

    /// `UInt<bits: 12>(x)` and `Int<range: 1..=65535>(x)`: a conversion
    /// (spec 2.1). `None` for any other call.
    pub(crate) fn int_type_call(
        &mut self,
        name: &str,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
    ) -> Option<Ty> {
        if !matches!(name, "Int" | "UInt") || generics.is_empty() {
            return None;
        }
        let nums: Vec<&TypeExpr> = generics.iter().collect();
        for g in &nums {
            if !matches!(g.kind, TypeKind::Num { .. }) {
                self.error(
                    g.span,
                    format!("`{name}` takes no type: write `{name}<bits: 24>` (spec 2.1)"),
                );
            }
        }
        match self.int_type(name, &nums, span) {
            Ty::Prim(p) => Some(self.convert(p, args, span)),
            t => Some(t),
        }
    }

    /// `try UInt<bits: 12>(x)`, `try Int8(x)`, or one with `catch`: a
    /// number converted to a whole-number type fails with a `SizeError`
    /// when it doesn't fit, where it would panic without them (spec 2.1).
    /// Text read as a number keeps its `ParseError`.
    pub(crate) fn checked_int_conversion(
        &mut self,
        name: &str,
        expr: &Expr,
        expected: Option<&Ty>,
    ) -> Option<Ty> {
        let whole = Prim::from_name(name).is_some_and(Prim::is_int) || name == "UInt";
        if !whole {
            return None;
        }
        match self.infer(expr, expected) {
            // A constant the compiler reads, and which fits, can't fail:
            // `Int("2") catch 0` keeps its warning (spec 2.1, 5.3).
            Ty::Prim(p) if p.is_int() && constant_fits(expr, p) => None,
            made @ Ty::Prim(p) if p.is_int() => {
                let failing = Ty::Result(
                    Box::new(made),
                    Box::new(Ty::Builtin("SizeError", Vec::new())),
                );
                self.expr_types.push((expr.span, failing.clone()));
                Some(failing)
            }
            other => Some(other),
        }
    }
}

/// Whether the one argument of conversion call `call` is a constant the
/// compiler reads (a whole number, a plain text literal or a float
/// literal) whose value fits whole-number type `p`.
fn constant_fits(call: &Expr, p: Prim) -> bool {
    let ExprKind::Call { args, .. } = &call.kind else {
        return false;
    };
    let [arg] = args.as_slice() else {
        return false;
    };
    let value = match &arg.value.kind {
        ExprKind::Str(parts) => match parts.as_slice() {
            [StrPart::Text(t)] => t.parse::<num_bigint::BigInt>().ok(),
            _ => None,
        },
        ExprKind::Float(f) => f
            .replace('_', "")
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite() && x.abs() < 9.0e15)
            .map(|x| num_bigint::BigInt::from(x.trunc() as i64)),
        _ => crate::constants::const_int(&arg.value),
    };
    value.is_some_and(|v| crate::constants::fits(&v, p))
}

impl Checker<'_> {
    /// `p & 1`, `~p`, `p << 2` on a range: an error, as a range is about
    /// values, not bits (spec 2.1). Whether it was one.
    pub(crate) fn no_bits_on_range(&mut self, e: &Expr, t: &Ty, op: &str) -> bool {
        let Ty::Prim(p @ Prim::Ranged(..)) = self.inf.resolve(t) else {
            return false;
        };
        let name = p.name();
        let first = if op == "~" {
            "~Int(x)".to_string()
        } else {
            format!("Int(x) {op} …")
        };
        self.error(
            e.span,
            format!("`{op}` works on bits, and `{name}` is a range: convert it first, as `{first}`, or use an `Int<bits: n>` (spec 2.1)"),
        );
        true
    }

    /// The value of `x = v` or `x op= v`: in `p += 1` on a range, the 1
    /// need only be an `Int`, as the result is checked (spec 2.1).
    pub(crate) fn check_assigned(&mut self, value: &Expr, slot: &Ty, op: AssignOp) {
        self.range_operand = op != AssignOp::Set
            && matches!(self.inf.resolve(slot), Ty::Prim(Prim::Ranged(..)))
            && crate::constants::const_int(value).is_some();
        self.check_expect(value, slot);
        self.range_operand = false;
    }

    /// `UInt` alone: it is always written with its bits.
    pub(crate) fn bare_uint(&mut self, span: Span) -> Ty {
        self.error(
            span,
            "`UInt` is written with its width: `UInt<bits: 32>`, or `UInt32` (spec 2.1)",
        );
        Ty::Prim(Prim::UInt64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_and_ranges_have_their_names_and_limits() {
        let cases: [(Prim, &str, (i128, i128)); 8] = [
            (bits_type(12, false), "UInt<bits: 12>", (0, 4095)),
            (
                bits_type(24, true),
                "Int<bits: 24>",
                (-8_388_608, 8_388_607),
            ),
            (bits_type(1, true), "Int<bits: 1>", (-1, 0)),
            (bits_type(1, false), "UInt<bits: 1>", (0, 1)),
            (bits_type(63, false), "UInt<bits: 63>", (0, i64::MAX.into())),
            (bits_type(8, true), "Int8", (-128, 127)),
            (bits_type(64, false), "UInt64", (0, u64::MAX.into())),
            (Prim::Ranged(-5, 9), "Int<range: -5..=9>", (-5, 9)),
        ];
        for (p, name, range) in cases {
            assert_eq!(p.name(), name);
            assert_eq!(p.int_range(), Some(range), "{name}");
            assert!(p.is_int(), "{name}");
        }
        assert_eq!(bits_type(64, true), Prim::Int);
        assert!(!Prim::Int8.is_narrow() && Prim::Bits(12, false).is_narrow());
    }
}
