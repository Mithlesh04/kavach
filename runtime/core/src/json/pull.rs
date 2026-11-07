//! Reading JSON text piece by piece, in the order a decoder asks for the
//! pieces: `json.decode<T>` reads text straight into values of `T` with
//! no tree in between. It accepts exactly what `parse` accepts, with the
//! same limits, so decoding never takes a document that parsing rejects.
//! Its errors carry no detail: a decoder that gets one decodes again
//! through `parse`, which says what went wrong and where.

use super::{Build, Error, Limits, MAX_DEPTH, Parser, Tree, duplicate, value_at};
use std::borrow::Cow;
use std::collections::HashSet;

/// JSON text being read.
pub struct Pull<'a> {
    p: Parser<'a>,
    limits: Limits,
    /// The containers open now.
    depth: usize,
}

/// The keys of an object read so far, to find duplicates.
#[derive(Default)]
pub struct Keys<'a> {
    names: Vec<Cow<'a, str>>,
    seen: Option<HashSet<Cow<'a, str>>>,
}

impl<'a> Pull<'a> {
    pub fn new(text: &'a str, limits: &Limits) -> Result<Pull<'a>, Error> {
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
        Ok(Pull {
            p,
            limits: *limits,
            depth: 0,
        })
    }

    /// The first byte of the next value: `n`, `t`, `f`, `"`, `[`, `{`, a
    /// digit or `-` (anything else is not a value).
    pub fn peek(&self) -> Option<u8> {
        self.p.peek()
    }

    pub fn null(&mut self) -> Result<(), Error> {
        self.p.word("null")
    }

    pub fn bool(&mut self) -> Result<bool, Error> {
        match self.peek() {
            Some(b't') => self.p.word("true").map(|_| true),
            _ => self.p.word("false").map(|_| false),
        }
    }

    /// A number: `Tree::Int`, `Tree::Float` or `Tree::Number`.
    pub fn number(&mut self) -> Result<Tree, Error> {
        self.p.number()
    }

    /// A number's text, and whether it is written without a fraction or
    /// exponent: for a reader that parses it into the type it wants.
    pub fn number_text(&mut self) -> Result<(&'a str, bool), Error> {
        self.p.number_text()
    }

    pub fn text(&mut self) -> Result<Cow<'a, str>, Error> {
        if self.peek() != Some(b'"') {
            return Err(self.p.syntax("expected text"));
        }
        self.p.string()
    }

    /// Any one value, made by `b` (as `parse_with` makes it).
    pub fn value<B: Build>(&mut self, b: &mut B) -> Result<B::Value, Error> {
        value_at(&mut self.p, &self.limits, self.depth, false, b)
    }

    /// Reads past one value, checking it as `parse` would.
    pub fn skip(&mut self) -> Result<(), Error> {
        self.value(&mut Skip)
    }

    fn open(&mut self, bracket: u8) -> Result<(), Error> {
        if self.peek() != Some(bracket) {
            return Err(self.p.syntax("expected a container"));
        }
        if self.depth >= self.limits.max_depth.min(MAX_DEPTH) {
            return Err(Error::TooDeep(self.limits.max_depth.min(MAX_DEPTH)));
        }
        self.depth += 1;
        self.p.at += 1;
        self.p.space();
        Ok(())
    }

    /// `[`: then `item` before each item.
    pub fn array(&mut self) -> Result<(), Error> {
        self.open(b'[')
    }

    /// Whether another item follows (the first one when `first`); at the
    /// end, reads the `]`.
    pub fn item(&mut self, first: bool) -> Result<bool, Error> {
        if first {
            if self.peek() == Some(b']') {
                self.p.at += 1;
                return Ok(self.close());
            }
            return Ok(true);
        }
        self.p.space();
        match self.p.next_byte() {
            Some(b',') => {
                self.p.space();
                Ok(true)
            }
            Some(b']') => Ok(self.close()),
            _ => Err(self.p.syntax_before("expected `,` or `]` in an array")),
        }
    }

    /// `{`: then `field` before each field.
    pub fn object(&mut self) -> Result<(), Error> {
        self.open(b'{')
    }

    /// The next field's key (with its `:` read), or `None` at the `}`.
    /// `keys` holds this object's keys so far.
    pub fn field(&mut self, keys: &mut Keys<'a>) -> Result<Option<Cow<'a, str>>, Error> {
        if keys.names.is_empty() {
            if self.peek() == Some(b'}') {
                self.p.at += 1;
                self.close();
                return Ok(None);
            }
        } else {
            if keys.names.len() > self.limits.max_fields {
                return Err(Error::TooLarge(self.limits.max_fields));
            }
            self.p.space();
            match self.p.next_byte() {
                Some(b',') => self.p.space(),
                Some(b'}') => {
                    self.close();
                    return Ok(None);
                }
                _ => return Err(self.p.syntax_before("expected `,` or `}` in an object")),
            }
        }
        let name = self.p.key()?;
        if duplicate(&keys.names, &mut keys.seen, &name) {
            return Err(self.p.syntax("a key that appears twice"));
        }
        keys.names.push(name.clone());
        Ok(Some(name))
    }

    /// After `]` or `}` was read; `false`, for `item`.
    fn close(&mut self) -> bool {
        self.depth -= 1;
        false
    }

    /// Checks that nothing but spaces follows the value read.
    pub fn end(&mut self) -> Result<(), Error> {
        self.p.space();
        if self.p.at < self.p.s.len() {
            return Err(self.p.syntax("unexpected text after the value"));
        }
        Ok(())
    }
}

/// Builds nothing: for `skip`.
struct Skip;

impl Build for Skip {
    type Value = ();
    type Key = ();
    fn null(&mut self) {}
    fn bool(&mut self, _: bool) {}
    fn int(&mut self, _: i64) {}
    fn float(&mut self, _: f64, _: &str) {}
    fn number(&mut self, _: &str) {}
    fn text(&mut self, _: &str) {}
    fn key(&mut self, _: &str) {}
    fn array(&mut self, items: std::vec::Drain<'_, ()>) {
        items.for_each(drop);
    }
    fn object(&mut self, fields: std::vec::Drain<'_, ((), ())>) {
        fields.for_each(drop);
    }
}
