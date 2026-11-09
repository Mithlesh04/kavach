//! Conversions between built-in types, one form for all: `T(value)`
//! (spec 2.1). Text read as a number, `Bool` or `Char` can fail; a
//! literal is read here, so `Int("2")` can't fail and `Int("abc")` is an
//! error now rather than at run time.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::*;
use num_bigint::BigInt;

/// Whether `text`, read whole with spaces around it allowed, is a valid
/// `p`. The run time reads it the same way (`interp`, `rt_numbers.rs`).
pub(crate) fn reads_as(text: &str, p: Prim) -> bool {
    let s = text.trim();
    match p {
        Prim::Float | Prim::Float32 => s.parse::<f64>().is_ok(),
        Prim::Decimal => is_decimal(s),
        Prim::BigInt => s.parse::<BigInt>().is_ok(),
        Prim::Bool => matches!(s, "true" | "false"),
        Prim::Char => text.chars().count() == 1,
        p if p.is_int() => s
            .parse::<BigInt>()
            .is_ok_and(|n| crate::constants::fits(&n, p)),
        _ => false,
    }
}

/// `-12.50`, `1_000`, `2.5e-3`: what `Decimal` reads.
fn is_decimal(s: &str) -> bool {
    let s = s.replace('_', "");
    let (number, exponent) = match s.split_once(['e', 'E']) {
        Some((n, e)) => (n.to_string(), Some(e)),
        None => (s, None),
    };
    let digits = number.strip_prefix(['-', '+']).unwrap_or(&number);
    let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let all = |t: &str| t.chars().all(|c| c.is_ascii_digit());
    !(int.is_empty() && frac.is_empty())
        && all(int)
        && all(frac)
        && exponent.is_none_or(|e| e.parse::<i32>().is_ok())
}

/// The text of a string literal with no `{…}` in it.
fn literal_text(e: &Expr) -> Option<String> {
    let ExprKind::Str(parts) = &e.kind else {
        return None;
    };
    parts
        .iter()
        .map(|p| match p {
            StrPart::Text(t) => Some(t.as_str()),
            StrPart::Expr(_) => None,
        })
        .collect()
}

impl Checker<'_> {
    /// `T(value)` for built-in type `p`.
    pub(crate) fn convert(&mut self, p: Prim, args: &[Arg], span: Span) -> Ty {
        let name = p.name();
        if args.len() != 1 {
            self.error(span, format!("`{name}` takes one argument"));
        }
        let Some(arg) = args.first() else {
            return Ty::Error;
        };
        let t = self.value(&arg.value, None);
        let from = self.inf.resolve(&t);
        let whole = |q: Prim| q.is_int() || q == Prim::BigInt;
        let ok = match (&from, p) {
            (_, Prim::String) => true,
            (Ty::Prim(Prim::String), q)
                if q.is_numeric() || matches!(q, Prim::Bool | Prim::Char) =>
            {
                return self.convert_text(&arg.value, p);
            }
            (Ty::Prim(Prim::Char), q) => whole(q),
            (Ty::Prim(q), Prim::Char) => whole(*q),
            (Ty::Prim(q), to) => q.is_numeric() && to.is_numeric(),
            (Ty::Var(_) | Ty::Error | Ty::Never, _) => true,
            _ => false,
        };
        if !ok {
            let shown = self.show(&t);
            self.error(span, format!("can't convert `{shown}` to `{name}`"));
        }
        Ty::Prim(p)
    }

    /// `T(text)`: a failing call, or for a literal the value it reads.
    fn convert_text(&mut self, text: &Expr, p: Prim) -> Ty {
        let name = p.name();
        if let Some(literal) = literal_text(text) {
            if !reads_as(&literal, p) {
                self.error(
                    text.span,
                    format!("`{literal}` is not a valid {name}, so `{name}(\"{literal}\")` would always fail"),
                );
            }
            return Ty::Prim(p);
        }
        Ty::Result(
            Box::new(Ty::Prim(p)),
            Box::new(Ty::Builtin("ParseError", Vec::new())),
        )
    }
}
