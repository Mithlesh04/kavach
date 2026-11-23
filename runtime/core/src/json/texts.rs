// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `json.decode_texts<T>(names, texts)` (spec 2.12): each field's text
//! read as its type reads text, into the tree decoding then takes, so the
//! value is made, with its defaults and invariants, as `json.decode`
//! makes it. The interpreter and native code both read texts here.

use super::Tree;

/// How a field reads its text.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextKind {
    /// A whole number of any type: `Int(s)`.
    Whole,
    Float,
    Float32,
    Decimal,
    Bool,
    /// Text as JSON holds it: a `String`, a `Char`, a date, a category.
    Text,
}

/// What a text that doesn't fit its field wanted (`JsonProblem`'s
/// `expected`): the words `std.table`'s CSV errors use.
pub mod wanted {
    pub const WHOLE: &str = "a whole number";
    pub const NUMBER: &str = "a number";
    pub const BOOL: &str = "`true` or `false`";
}

/// `text` read as `kind` reads it: strictly, as `Int(s)`, `Float(s)` and
/// `Decimal(s)` read text, but with no spaces around a number; `true` or
/// `false` for a `Bool`. `NaN`, `inf` and `-inf` are floats, as `print`
/// writes them; a `Float32` is read straight to single precision.
pub fn text_tree(kind: TextKind, text: &str) -> Result<Tree, &'static str> {
    let spaced = text.trim() != text || text.is_empty();
    match kind {
        TextKind::Text => Ok(Tree::Text(text.to_string())),
        TextKind::Bool => match text {
            "true" => Ok(Tree::Bool(true)),
            "false" => Ok(Tree::Bool(false)),
            _ => Err(wanted::BOOL),
        },
        _ if spaced => Err(match kind {
            TextKind::Whole => wanted::WHOLE,
            _ => wanted::NUMBER,
        }),
        TextKind::Whole => {
            let n: num_bigint::BigInt = text.parse().map_err(|_| wanted::WHOLE)?;
            Ok(i64::try_from(&n).map_or_else(|_| Tree::Number(n.to_string()), Tree::Int))
        }
        TextKind::Float => text
            .parse::<f64>()
            .map(Tree::Float)
            .map_err(|_| wanted::NUMBER),
        TextKind::Float32 => (text.parse::<f32>())
            .map(|f| Tree::Float(f64::from(f)))
            .map_err(|_| wanted::NUMBER),
        TextKind::Decimal => crate::AnyDecimal::parse(text)
            .map(|d| Tree::Number(d.to_string()))
            .ok_or(wanted::NUMBER),
    }
}

#[cfg(test)]
mod tests {
    use super::{TextKind, Tree, text_tree, wanted};

    #[test]
    fn texts_are_read_strictly() {
        let cases = [
            (TextKind::Whole, "42", Ok(Tree::Int(42))),
            (TextKind::Whole, "-7", Ok(Tree::Int(-7))),
            (
                TextKind::Whole,
                "99999999999999999999",
                Ok(Tree::Number("99999999999999999999".into())),
            ),
            (TextKind::Whole, " 12", Err(wanted::WHOLE)),
            (TextKind::Whole, "1,299", Err(wanted::WHOLE)),
            (TextKind::Whole, "1.5", Err(wanted::WHOLE)),
            (TextKind::Float, "1.5", Ok(Tree::Float(1.5))),
            (TextKind::Float, "inf", Ok(Tree::Float(f64::INFINITY))),
            (TextKind::Float, "-inf", Ok(Tree::Float(f64::NEG_INFINITY))),
            (TextKind::Float, "1.5 ", Err(wanted::NUMBER)),
            (TextKind::Float, "", Err(wanted::NUMBER)),
            (TextKind::Float32, "0.1", Ok(Tree::Float(f64::from(0.1f32)))),
            (TextKind::Decimal, "1.", Ok(Tree::Number("1".into()))),
            (TextKind::Decimal, "x", Err(wanted::NUMBER)),
            (TextKind::Bool, "true", Ok(Tree::Bool(true))),
            (TextKind::Bool, "True", Err(wanted::BOOL)),
            (TextKind::Text, " a ", Ok(Tree::Text(" a ".into()))),
        ];
        for (kind, text, want) in cases {
            assert_eq!(text_tree(kind, text), want, "{kind:?} {text:?}");
        }
        assert!(matches!(text_tree(TextKind::Float, "NaN"), Ok(Tree::Float(f)) if f.is_nan()));
    }
}
