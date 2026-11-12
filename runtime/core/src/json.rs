//! JSON text (spec 2.12): one parser and one writer for the interpreter
//! and native code, so both read and write exactly the same.
//!
//! The parser keeps its own stack instead of recursing. Nesting is
//! limited (at most `MAX_DEPTH`), because freeing and printing values
//! recurse. It is strict: what RFC 8259 allows, and nothing else, with the limits
//! and errors of spec 2.12; with `comments`, comments and trailing commas
//! too (JSONC).
//!
//! A number keeps its exact digits (decision 0071): one that writes back
//! exactly as an `i64` or an `f64` is kept as one, which is fast; any
//! other keeps its text.

/// What decoding wanted where a value didn't fit (a `JsonProblem`'s
/// `expected`), the same in the interpreter and native code.
pub mod expected {
    pub const NUMBER: &str = "a number";
    pub const WHOLE: &str = "a whole number";
    pub const BOOL: &str = "true or false";
    pub const TEXT: &str = "text";
    pub const CHAR: &str = "text of one character";
    pub const ARRAY: &str = "an array";
    pub const OBJECT: &str = "an object";
    /// For a field the type doesn't have (decision 0096).
    pub const NO_FIELD: &str = "no field of this name";

    /// For an integer field of type `name` (`Int`, `UInt8`…).
    pub fn integer(name: &str) -> String {
        format!("a whole number that fits `{name}`")
    }

    /// For a `Float` or `Float32` field, given a number past its range.
    pub fn float(name: &str) -> String {
        format!("a number that fits `{name}`")
    }

    pub fn items(n: usize) -> String {
        format!("an array of {n} items")
    }

    pub fn field(name: &str) -> String {
        format!("a field `{name}`")
    }

    pub fn variant(union: &str) -> String {
        format!("a variant of `{union}`")
    }
}

/// Where a part of a document is: `items[2].price` (`""` for the whole).
pub fn path_field(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}

pub fn path_item(path: &str, i: usize) -> String {
    format!("{path}[{i}]")
}

/// A parsed JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Tree {
    Null,
    Bool(bool),
    /// A number written without a fraction or exponent, as an `i64`
    /// writes it.
    Int(i64),
    /// A number written as an `f64` writes it (`format_float`).
    Float(f64),
    /// Any other number, by its text: too big, too precise, or written
    /// another way (`1e3`, `-0`).
    Number(String),
    Text(String),
    Array(Vec<Tree>),
    /// Fields in the order written.
    Object(Vec<(String, Tree)>),
}

/// How much a document may hold (spec 2.12).
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_depth: usize,
    pub max_bytes: usize,
    pub max_fields: usize,
    /// `json.parse(text, comments: true)`: `//` and `/* */` comments, and
    /// a trailing comma in arrays and objects (JSONC).
    pub comments: bool,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_depth: 128,
            max_bytes: 64_000_000,
            max_fields: 100_000,
            comments: false,
        }
    }
}

/// Why a text is not an acceptable JSON document. Lines and columns
/// count from 1; columns count characters.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Syntax {
        line: usize,
        column: usize,
        message: String,
    },
    TooDeep(usize),
    TooLarge(usize),
    DuplicateKey {
        key: String,
        line: usize,
        column: usize,
    },
}

/// What parsing makes values with: a `Tree` (`parse`), or the values of
/// a running program directly, with no tree in between. A value or key
/// that parsing can't use after all (an error later in the text) is
/// handed back to `discard`.
pub trait Build {
    type Value;
    type Key;
    /// Takes every number by its text (`number`), so the parser needn't
    /// work out an `i64` or `f64` that would be thrown away.
    const KEEPS_DIGITS: bool = false;
    /// A number each key's text has one of: two keys with the same number
    /// are the same text, so finding duplicates compares no text. `None`
    /// when this builder has no such number.
    fn key_id(&self, _key: &Self::Key) -> Option<u64> {
        None
    }
    fn null(&mut self) -> Self::Value;
    fn bool(&mut self, b: bool) -> Self::Value;
    fn int(&mut self, n: i64) -> Self::Value;
    /// A number written as `text`, which is how `f` writes too.
    fn float(&mut self, f: f64, text: &str) -> Self::Value;
    /// A number kept by its text (`Tree::Number`).
    fn number(&mut self, text: &str) -> Self::Value;
    fn text(&mut self, s: &str) -> Self::Value;
    fn key(&mut self, s: &str) -> Self::Key;
    fn array(&mut self, items: std::vec::Drain<'_, Self::Value>) -> Self::Value;
    fn object(&mut self, fields: std::vec::Drain<'_, (Self::Key, Self::Value)>) -> Self::Value;
    fn discard(&mut self, _value: Self::Value) {}
    fn discard_key(&mut self, _key: Self::Key) {}
}

/// Builds a `Tree`.
struct TreeBuild;

impl Build for TreeBuild {
    type Value = Tree;
    type Key = String;
    fn null(&mut self) -> Tree {
        Tree::Null
    }
    fn bool(&mut self, b: bool) -> Tree {
        Tree::Bool(b)
    }
    fn int(&mut self, n: i64) -> Tree {
        Tree::Int(n)
    }
    fn float(&mut self, f: f64, _: &str) -> Tree {
        Tree::Float(f)
    }
    fn number(&mut self, text: &str) -> Tree {
        Tree::Number(text.to_string())
    }
    fn text(&mut self, s: &str) -> Tree {
        Tree::Text(s.to_string())
    }
    fn key(&mut self, s: &str) -> String {
        s.to_string()
    }
    fn array(&mut self, items: std::vec::Drain<'_, Tree>) -> Tree {
        Tree::Array(items.collect())
    }
    fn object(&mut self, fields: std::vec::Drain<'_, (String, Tree)>) -> Tree {
        Tree::Object(fields.collect())
    }
}

/// The deepest nesting accepted whatever `max_depth` says: freeing,
/// printing and converting values recurse, and must not run out of stack.
pub const MAX_DEPTH: usize = 10_000;

/// Objects with more fields than this check duplicates with a set.
const SCAN: usize = 16;

/// Whether `text` is exactly one JSON number (RFC 8259): what a
/// `Json.Number`'s digits must be to be written as a number. A scan of its
/// own, not the parser, so a program that only prints JSON stays small.
pub fn is_number(text: &str) -> bool {
    let s = text.as_bytes();
    let mut at = usize::from(s.first() == Some(&b'-'));
    let digits = |at: &mut usize| {
        let start = *at;
        while s.get(*at).is_some_and(u8::is_ascii_digit) {
            *at += 1;
        }
        *at - start
    };
    let first = s.get(at).copied();
    let whole = digits(&mut at);
    if whole == 0 || (first == Some(b'0') && whole > 1) {
        return false;
    }
    if s.get(at) == Some(&b'.') {
        at += 1;
        if digits(&mut at) == 0 {
            return false;
        }
    }
    if matches!(s.get(at), Some(b'e' | b'E')) {
        at += 1;
        if matches!(s.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        if digits(&mut at) == 0 {
            return false;
        }
    }
    at == s.len()
}

/// The largest power of ten a number is read as exactly (spec 2.12), so a
/// short text like `1e999999999` can't make a huge `Decimal`.
pub const EXPONENT_LIMIT: i64 = 10_000;

/// Whether the exponent of number `text`, if it has one, is within
/// `EXPONENT_LIMIT` either way.
pub fn exponent_fits(text: &str) -> bool {
    match text.find(['e', 'E']) {
        None => true,
        Some(i) => text[i + 1..]
            .parse::<i64>()
            .is_ok_and(|e| (-EXPONENT_LIMIT..=EXPONENT_LIMIT).contains(&e)),
    }
}

/// The digits a number of `tree` is written with (`None` for any other
/// value): what a `Json.Number` holds (decision 0071).
pub fn digits(tree: &Tree) -> Option<String> {
    match tree {
        Tree::Int(n) => Some(n.to_string()),
        Tree::Float(f) if f.is_finite() => Some(crate::format_float(*f)),
        Tree::Number(text) => Some(text.clone()),
        _ => None,
    }
}

pub fn parse(text: &str, limits: &Limits) -> Result<Tree, Error> {
    parse_with(text, limits, &mut TreeBuild)
}

mod parse;
pub use parse::parse_with;
use parse::{Seen, duplicate, value_at};

mod pull;
pub use pull::{Keys, Pull};

mod read;
use read::Parser;

mod write;
pub use write::{write, write_int, write_text};

#[cfg(test)]
mod tests;
