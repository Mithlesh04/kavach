//! A parsed JSON document on a tape (decision 0032): every value is one
//! word, and an array or object is a run of words, all in one list of
//! words per document. Strings and numbers are their place in the parsed
//! text (the document keeps a copy), or in a side text for those with
//! escapes. A `Json` array or object read from a document is a small
//! object pointing into it (`Data::Json`); its list or map is made only
//! when a program asks for it, and kept.
//!
//! A word: its kind in the top 4 bits. A string or number: bit 59 says
//! it is in the side text; bit 58 that its place and length are too big
//! for the word and are in two tape words at the index below; otherwise
//! its place is in bits 24..58 and its length in bits 0..24. An array or
//! object: the index of its run, whose first word is its length.

use crate::heap::{Data, alloc_slots, new_obj};
use crate::shapes::{new_text, shape};
use kavach_kir::{Shape, Type};
use kavach_runtime_core::json::Tree;
use std::sync::{Arc, OnceLock};

pub const NULL: u64 = 0;
pub const FALSE: u64 = 1 << 60;
pub const TRUE: u64 = 2 << 60;
pub const NUMBER: u64 = 3;
pub const TEXT: u64 = 4;
pub const ARRAY: u64 = 5;
pub const OBJECT: u64 = 6;
pub const SIDE: u64 = 1 << 59;
const LONG: u64 = 1 << 58;
const LEN_BITS: u32 = 24;
const OFF_BITS: u32 = 34;
const INDEX: u64 = (1 << 58) - 1;

/// The kind of word `w`: `NUMBER`, `TEXT`, `ARRAY`, `OBJECT`, or 0, 1, 2
/// for `null`, `false`, `true`.
pub fn kind(w: u64) -> u64 {
    w >> 60
}

/// A word for a string of `kind` at `off`, `len` bytes long, if they fit.
pub fn short(kind: u64, side: u64, off: usize, len: usize) -> Option<u64> {
    (len < 1 << LEN_BITS && off < 1 << OFF_BITS)
        .then_some(kind << 60 | side | (off as u64) << LEN_BITS | len as u64)
}

/// A word for a string of `kind` whose place and length are at tape
/// index `i`.
pub fn long(kind: u64, i: usize) -> u64 {
    kind << 60 | LONG | i as u64
}

/// A word for an array or object whose run starts at index `i`.
pub fn container(kind: u64, i: usize) -> u64 {
    kind << 60 | i as u64
}

/// One parsed document.
pub struct Doc {
    pub text: Box<str>,
    pub side: Box<str>,
    pub nodes: Box<[u64]>,
}

impl Doc {
    /// The bytes of string or number `w`.
    pub fn bytes(&self, w: u64) -> &[u8] {
        let (side, off, len) = if w & LONG != 0 {
            let i = (w & INDEX) as usize;
            let at = self.nodes[i];
            (
                at & SIDE != 0,
                (at & !SIDE) as usize,
                self.nodes[i + 1] as usize,
            )
        } else {
            let off = (w >> LEN_BITS) & ((1 << OFF_BITS) - 1);
            (
                w & SIDE != 0,
                off as usize,
                (w & ((1 << LEN_BITS) - 1)) as usize,
            )
        };
        let src = if side { &self.side } else { &self.text };
        &src.as_bytes()[off..off + len]
    }

    /// The text of string or number `w`.
    #[allow(unsafe_code)]
    pub fn str(&self, w: u64) -> &str {
        // SAFETY: the document's text and side text are `str`s, and a
        // word covers a whole string or number of them (`Builder`), which
        // starts and ends between characters.
        unsafe { std::str::from_utf8_unchecked(self.bytes(w)) }
    }

    /// Whether string `w` is `key`, its length looked at first.
    pub fn is(&self, w: u64, key: &[u8]) -> bool {
        (w & LONG != 0 || (w & ((1 << LEN_BITS) - 1)) as usize == key.len()) && self.bytes(w) == key
    }

    /// The items of array `w`, or the keys and values, in turn, of
    /// object `w`.
    pub fn run(&self, w: u64) -> &[u64] {
        let i = (w & INDEX) as usize;
        let n = self.nodes[i] as usize;
        let per = if kind(w) == OBJECT { 2 } else { 1 };
        &self.nodes[i + 1..i + 1 + n * per]
    }

    /// Value `w` as a tree, for code that works on trees.
    pub fn tree(&self, w: u64) -> Tree {
        match kind(w) {
            1 => Tree::Bool(false),
            2 => Tree::Bool(true),
            NUMBER => Tree::Number(self.str(w).to_string()),
            TEXT => Tree::Text(self.str(w).to_string()),
            ARRAY => Tree::Array(self.run(w).iter().map(|v| self.tree(*v)).collect()),
            OBJECT => Tree::Object(
                (self.run(w).as_chunks::<2>().0.iter())
                    .map(|[k, v]| (self.str(*k).to_string(), self.tree(*v)))
                    .collect(),
            ),
            _ => Tree::Null,
        }
    }
}

/// What `Json` values are made of: the union's type, each variant's tag
/// (in the order of `VARIANTS`) and the types of their fields.
struct Types {
    json: Type,
    tags: [u32; 6],
    text: Type,
    digits: Type,
    list: Type,
    map: Type,
    key: Type,
}

/// The variants of `Json`, in the order `stdlib/json/json.kav` declares.
const VARIANTS: [&str; 6] = ["Null", "Boolean", "Number", "Text", "Array", "Object"];

fn types_of(json: Type) -> Types {
    let mut t = Types {
        json,
        tags: [0; 6],
        text: Type::Unit,
        digits: Type::Unit,
        list: Type::Unit,
        map: Type::Unit,
        key: Type::Unit,
    };
    if let Shape::Union { variants, .. } = shape(json) {
        for (i, name) in VARIANTS.iter().enumerate() {
            let Some(tag) = variants.iter().position(|v| v.name == *name) else {
                continue;
            };
            t.tags[i] = tag as u32;
            match (*name, variants[tag].fields.first().map(|f| f.ty)) {
                ("Text", Some(f)) => t.text = f,
                ("Number", Some(f)) => t.digits = f,
                ("Array", Some(f)) => t.list = f,
                ("Object", Some(f)) => t.map = f,
                _ => {}
            }
        }
    }
    if let Shape::Map(k, _) = shape(t.map) {
        t.key = *k;
    }
    t
}

/// The types of `Json` values of type `json`: worked out once, as a
/// program has one `Json` type.
fn types(json: Type) -> &'static Types {
    static ONE: OnceLock<Types> = OnceLock::new();
    let t = ONE.get_or_init(|| types_of(json));
    if t.json == json {
        return t;
    }
    Box::leak(Box::new(types_of(json)))
}

/// The tag of each variant of `Json` type `json`, in the order of
/// `VARIANTS`.
pub fn tags(json: Type) -> [u32; 6] {
    types(json).tags
}

/// `Json.Null`, of type `json`.
pub fn null(json: Type) -> i64 {
    crate::heap::alloc_zeroed(json, types(json).tags[0], 0)
}

/// Value `w` of `doc` as a `Json` of type `json`: an array or object as
/// an object pointing into the document, anything else as it is.
pub fn value(doc: &Arc<Doc>, w: u64, json: Type) -> i64 {
    let t = types(json);
    let variant = |i: usize, slot: Option<u64>| {
        let p = crate::heap::alloc_zeroed(json, t.tags[i], slot.is_some() as usize);
        if let Some(s) = slot {
            crate::heap::slots_mut(p)[0] = s;
        }
        p
    };
    match kind(w) {
        1 => variant(1, Some(0)),
        2 => variant(1, Some(1)),
        NUMBER => variant(2, Some(new_text(doc.str(w), t.digits) as u64)),
        TEXT => variant(3, Some(new_text(doc.str(w), t.text) as u64)),
        k @ (ARRAY | OBJECT) => {
            crate::shapes::JSON_FIELD.get_or_init(|| field);
            let tag = t.tags[if k == ARRAY { 4 } else { 5 }];
            new_obj(json, tag, Vec::new(), Data::Json(doc.clone(), w, 0))
        }
        _ => variant(0, None),
    }
}

/// The list or map of `Json` array or object `p` (on a tape), borrowed
/// from it: made the first time it is asked for, then kept.
pub fn field(p: i64) -> i64 {
    let (doc, w) = match &*crate::heap::data(p).borrow() {
        Data::Json(_, _, made) if *made != 0 => return *made,
        Data::Json(doc, w, _) => (doc.clone(), *w),
        _ => return 0,
    };
    let t = types(crate::heap::obj(p).ty());
    let made = if kind(w) == ARRAY {
        let items = doc.run(w).iter().map(|v| value(&doc, *v, t.json) as u64);
        alloc_slots(t.list, 0, items.collect())
    } else {
        let mut map = crate::rt_maps::kv_map_new(crate::rt::type_code(t.map));
        for [k, v] in doc.run(w).as_chunks::<2>().0 {
            let key = new_text(doc.str(*k), t.key);
            map = crate::rt_maps::kv_map_insert(map, key, value(&doc, *v, t.json));
        }
        map
    };
    if let Data::Json(_, _, slot) = &mut *crate::heap::data(p).borrow_mut() {
        *slot = made;
    }
    made
}

/// The document and word of `Json` value `p`, if it points into a tape.
pub fn on_tape(p: i64) -> Option<(Arc<Doc>, u64)> {
    match &*crate::heap::data(p).borrow() {
        Data::Json(doc, w, _) => Some((doc.clone(), *w)),
        _ => None,
    }
}
