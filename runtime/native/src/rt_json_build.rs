//! Parsing JSON onto a tape (decision 0032): the parser
//! (`kavach_runtime_core::json::parse_with`) hands each value to this
//! builder, which writes it as one word (`rt_json_tape.rs`) and each
//! array or object as a run of words on one growing list of words. A
//! document costs a few allocations however many values it holds; `Json`
//! values are made from it only when the program reads them.

use crate::rt_json_tape::{self as tape, Doc};
use kavach_runtime_core::json::Build;
use std::sync::Arc;

pub struct Builder {
    /// The text being parsed, when the document keeps a copy of it: a
    /// string or number that lies in it is kept as its place there.
    base: Option<(usize, usize)>,
    side: String,
    nodes: Vec<u64>,
}

impl Builder {
    /// A builder for a document whose text is `base`, which `finish`
    /// will be given a copy of; with `None`, every string is copied.
    pub fn new(base: Option<&str>) -> Builder {
        Builder {
            base: base.map(|s| (s.as_ptr() as usize, s.len())),
            side: String::new(),
            nodes: Vec::new(),
        }
    }

    /// The document, with `text` the copy of the text given to `new`
    /// (empty without one), and `root` its value.
    pub fn finish(self, text: Box<str>, root: u64) -> (Arc<Doc>, u64) {
        let doc = Doc {
            text,
            side: self.side.into_boxed_str(),
            nodes: self.nodes.into_boxed_slice(),
        };
        (Arc::new(doc), root)
    }

    /// A word for text `s` of `kind`: its place in the parsed text, or
    /// in the side text when it isn't there as it is (escapes).
    fn string(&mut self, kind: u64, s: &str) -> u64 {
        let at = s.as_ptr() as usize;
        let (side, off) = match self.base {
            Some((start, len)) if at >= start && at + s.len() <= start + len => (0, at - start),
            _ => {
                let off = self.side.len();
                self.side.push_str(s);
                (tape::SIDE, off)
            }
        };
        if let Some(w) = tape::short(kind, side, off, s.len()) {
            return w;
        }
        let i = self.nodes.len();
        self.nodes.push(side | off as u64);
        self.nodes.push(s.len() as u64);
        tape::long(kind, i)
    }
}

impl Build for Builder {
    type Value = u64;
    type Key = u64;
    const KEEPS_DIGITS: bool = true;

    fn null(&mut self) -> u64 {
        tape::NULL
    }

    fn bool(&mut self, b: bool) -> u64 {
        if b { tape::TRUE } else { tape::FALSE }
    }

    fn int(&mut self, n: i64) -> u64 {
        self.number(&n.to_string())
    }

    fn float(&mut self, _: f64, text: &str) -> u64 {
        self.number(text)
    }

    fn number(&mut self, text: &str) -> u64 {
        self.string(tape::NUMBER, text)
    }

    fn text(&mut self, s: &str) -> u64 {
        self.string(tape::TEXT, s)
    }

    fn key(&mut self, s: &str) -> u64 {
        self.string(tape::TEXT, s)
    }

    fn array(&mut self, items: std::vec::Drain<'_, u64>) -> u64 {
        let i = self.nodes.len();
        self.nodes.push(items.len() as u64);
        self.nodes.extend(items);
        tape::container(tape::ARRAY, i)
    }

    fn object(&mut self, fields: std::vec::Drain<'_, (u64, u64)>) -> u64 {
        let i = self.nodes.len();
        self.nodes.push(fields.len() as u64);
        for (k, v) in fields {
            self.nodes.push(k);
            self.nodes.push(v);
        }
        tape::container(tape::OBJECT, i)
    }
}
