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

use std::borrow::Cow;
use std::collections::HashSet;

/// What decoding wanted where a value didn't fit (`Mismatch`'s
/// `expected`), the same in the interpreter and native code.
pub mod expected {
    pub const NUMBER: &str = "a number";
    pub const WHOLE: &str = "a whole number";
    pub const BOOL: &str = "true or false";
    pub const TEXT: &str = "text";
    pub const CHAR: &str = "text of one character";
    pub const ARRAY: &str = "an array";
    pub const OBJECT: &str = "an object";

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

/// A container being filled: where its items, or its fields and their
/// names, start on the shared stacks; an object's key waiting for its
/// value, and its keys in a set once it is large.
enum Open<'a, K> {
    Array(usize),
    Object(usize, usize, Option<K>, Option<HashSet<Cow<'a, str>>>),
}

/// The values of every open container, one stack for all, so a container
/// costs no allocation of its own; and each field's key text, for
/// finding duplicates.
struct Stacks<'a, B: Build> {
    items: Vec<B::Value>,
    fields: Vec<(B::Key, B::Value)>,
    names: Vec<Cow<'a, str>>,
    open: Vec<Open<'a, B::Key>>,
}

/// The value written in `text`, made by `b`.
pub fn parse_with<B: Build>(text: &str, limits: &Limits, b: &mut B) -> Result<B::Value, Error> {
    if text.len() > limits.max_bytes {
        return Err(Error::TooLarge(limits.max_bytes));
    }
    let mut p = Parser {
        s: text.as_bytes(),
        text,
        at: 0,
        comments: limits.comments,
    };
    p.space();
    value_at(&mut p, limits, 0, true, b)
}

/// The value at `p`, inside `depth` open containers. With `whole`, it
/// must be all that is left of the text.
fn value_at<'a, B: Build>(
    p: &mut Parser<'a>,
    limits: &Limits,
    depth: usize,
    whole: bool,
    b: &mut B,
) -> Result<B::Value, Error> {
    let mut st = Stacks::<B> {
        items: Vec::new(),
        fields: Vec::new(),
        names: Vec::new(),
        open: Vec::new(),
    };
    let result = run(p, limits, depth, whole, b, &mut st);
    if result.is_err() {
        st.items.drain(..).for_each(|v| b.discard(v));
        for (k, v) in st.fields.drain(..) {
            b.discard_key(k);
            b.discard(v);
        }
        for o in st.open.drain(..) {
            if let Open::Object(_, _, Some(k), _) = o {
                b.discard_key(k);
            }
        }
    }
    result
}

fn run<'a, B: Build>(
    p: &mut Parser<'a>,
    limits: &Limits,
    depth: usize,
    whole: bool,
    b: &mut B,
    st: &mut Stacks<'a, B>,
) -> Result<B::Value, Error> {
    let max_depth = limits.max_depth.min(MAX_DEPTH);
    loop {
        // A value: a scalar, or the start of a container.
        let value = match p.peek() {
            Some(b'[') | Some(b'{') => {
                if depth + st.open.len() >= max_depth {
                    return Err(Error::TooDeep(max_depth));
                }
                let array = p.s[p.at] == b'[';
                p.at += 1;
                p.space();
                let close = if array { b']' } else { b'}' };
                if p.peek() == Some(close) {
                    p.at += 1;
                    if array {
                        b.array(Vec::new().drain(..))
                    } else {
                        b.object(Vec::new().drain(..))
                    }
                } else if array {
                    st.open.push(Open::Array(st.items.len()));
                    continue;
                } else {
                    let name = p.key()?;
                    let key = b.key(&name);
                    let starts = (st.fields.len(), st.names.len());
                    st.open
                        .push(Open::Object(starts.0, starts.1, Some(key), None));
                    st.names.push(name);
                    continue;
                }
            }
            Some(b'"') => {
                let s = p.string()?;
                b.text(&s)
            }
            _ => p.scalar(b)?,
        };
        // Put it in its container; close every container that ends here.
        let mut value = value;
        loop {
            let Some(top) = st.open.last_mut() else {
                if !whole {
                    return Ok(value);
                }
                p.space();
                if p.at < p.s.len() {
                    b.discard(value);
                    return Err(p.syntax("unexpected text after the value"));
                }
                return Ok(value);
            };
            p.space();
            match top {
                Open::Array(_) => {
                    st.items.push(value);
                    match p.next_byte() {
                        Some(b',') => {
                            p.space();
                            // JSONC: `[1, 2,]`.
                            if !(p.comments && p.peek() == Some(b']')) {
                                break;
                            }
                            p.at += 1;
                        }
                        Some(b']') => {}
                        _ => return Err(p.syntax_before("expected `,` or `]` in an array")),
                    }
                }
                Open::Object(start, names, key, seen) => {
                    let k = key
                        .take()
                        .expect("an object waiting for a value has its key");
                    st.fields.push((k, value));
                    let (start, names) = (*start, *names);
                    if st.fields.len() - start > limits.max_fields {
                        return Err(Error::TooLarge(limits.max_fields));
                    }
                    match p.next_byte() {
                        Some(b',') if p.comments && p.after_space() == Some(b'}') => {
                            // JSONC: `{"a": 1,}`.
                            p.space();
                            p.at += 1;
                        }
                        Some(b',') => {
                            p.space();
                            let at = p.at;
                            let name = p.key()?;
                            if duplicate(&st.names[names..], seen, &name) {
                                let (line, column) = p.place(at);
                                return Err(Error::DuplicateKey {
                                    key: name.into_owned(),
                                    line,
                                    column,
                                });
                            }
                            *key = Some(b.key(&name));
                            st.names.push(name);
                            break;
                        }
                        Some(b'}') => {}
                        _ => return Err(p.syntax_before("expected `,` or `}` in an object")),
                    }
                }
            }
            value = match st.open.pop() {
                Some(Open::Array(start)) => b.array(st.items.drain(start..)),
                Some(Open::Object(start, names, ..)) => {
                    st.names.truncate(names);
                    b.object(st.fields.drain(start..))
                }
                None => unreachable!("a container was open"),
            };
        }
    }
}

/// Whether `key` is already one of `names` (an object's keys so far): a
/// scan while the object is small, then a set of its keys.
fn duplicate<'a>(
    names: &[Cow<'a, str>],
    seen: &mut Option<HashSet<Cow<'a, str>>>,
    key: &str,
) -> bool {
    if names.len() < SCAN {
        return names.iter().any(|k| k == key);
    }
    let set = seen.get_or_insert_with(|| names.iter().cloned().collect());
    !set.insert(Cow::Owned(key.to_string()))
}

mod pull;
pub use pull::{Keys, Pull};

mod read;
use read::Parser;

mod write;
pub use write::{write, write_text};

#[cfg(test)]
mod tests;
