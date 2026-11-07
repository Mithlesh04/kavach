//! Parsing JSON straight into `Json` values (spec 2.12), with no tree in
//! between: the parser (`kavach_runtime_core::json::parse_with`) hands
//! each value to this builder as soon as it has read it. Maps are made at
//! their final size, and values that repeat are shared: `null`, `true`
//! and `false` are one object each, so are small integers, short text and
//! short numbers seen recently, and each key (with its hash kept). A
//! number is a `Number` holding its digits (decision 0071). A `Json` can't be
//! changed in place (spec 2.12), so sharing never shows.

use crate::display::hash_slot;
use crate::heap::{Data, alloc, alloc_slots, alloc_zeroed, release, retain, slots, slots_mut};
use crate::shapes::{new_text, shape, with_str};
use crate::table::Table;
use kavach_kir::{Shape, Type};
use kavach_runtime_core::json::Build;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// Maps keyed by text from outside, hashed with the seeded map hash.
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<crate::hashing::FastHasher>>;

/// Integers from 0 below this are shared.
const SMALL_INTS: usize = 256;

/// Text this long or shorter is shared while it stays in one of
/// `TEXT_SLOTS` slots.
const SHORT_TEXT: usize = 16;
const TEXT_SLOTS: usize = 1024;

/// The variants of `Json`, in the order `stdlib/json/json.kav` declares.
const VARIANTS: [&str; 6] = ["Null", "Boolean", "Number", "Text", "Array", "Object"];
const NUMBER: usize = 2;
const TEXT: usize = 3;

pub struct Builder {
    json: Type,
    /// The types of a `Text`'s text, a `Number`'s digits, an `Array`'s
    /// list, an `Object`'s map and its keys.
    text: Type,
    digits: Type,
    list: Type,
    map: Type,
    key: Type,
    /// Each variant's tag, in the order of `VARIANTS`.
    tags: [u32; 6],
    /// The shared `null`, `false` and `true`, once made; the builder
    /// holds one reference to each shared value.
    null: i64,
    bools: [i64; 2],
    /// The shared `Integer`s 0 to `SMALL_INTS - 1`, made when first seen.
    ints: Vec<i64>,
    /// Short `Text` and `Number` values seen recently, by hash: a slot is
    /// replaced when another lands in it.
    texts: Vec<i64>,
    numbers: Vec<i64>,
    /// Each key text seen so far, and its hash; the builder holds one
    /// reference to each.
    keys: FastMap<Box<str>, (i64, u64)>,
    /// The hash of each key handed out, by its text object.
    hashes: FastMap<i64, u64>,
}

impl Builder {
    pub fn new(json: Type) -> Builder {
        let mut b = Builder {
            json,
            text: Type::Unit,
            digits: Type::Unit,
            list: Type::Unit,
            map: Type::Unit,
            key: Type::Unit,
            tags: [0; 6],
            null: 0,
            bools: [0; 2],
            ints: Vec::new(),
            texts: Vec::new(),
            numbers: Vec::new(),
            keys: FastMap::default(),
            hashes: FastMap::default(),
        };
        if let Shape::Union { variants, .. } = shape(json) {
            for (i, name) in VARIANTS.iter().enumerate() {
                let Some(tag) = variants.iter().position(|v| v.name == *name) else {
                    continue;
                };
                b.tags[i] = tag as u32;
                let field = variants[tag].fields.first().map(|f| f.ty);
                match (*name, field) {
                    ("Text", Some(t)) => b.text = t,
                    ("Number", Some(t)) => b.digits = t,
                    ("Array", Some(t)) => b.list = t,
                    ("Object", Some(t)) => b.map = t,
                    _ => {}
                }
            }
        }
        if let Shape::Map(k, _) = shape(b.map) {
            b.key = *k;
        }
        b
    }

    /// Lets go of the shared `null` and keys.
    pub fn finish(self) {
        release(self.null);
        let shared = (self.bools.iter().chain(&self.ints))
            .chain(&self.texts)
            .chain(&self.numbers);
        shared.for_each(|p| release(*p));
        self.keys.values().for_each(|(k, _)| release(*k));
    }

    /// A `Text` or `Number` (variant `i`) holding `s`: short ones shared
    /// while they stay in their cache slot.
    fn shared(&mut self, i: usize, s: &str) -> u64 {
        let ty = if i == NUMBER { self.digits } else { self.text };
        if s.len() > SHORT_TEXT {
            return self.variant(i, new_text(s, ty) as u64);
        }
        let cache = if i == NUMBER {
            &mut self.numbers
        } else {
            &mut self.texts
        };
        if cache.is_empty() {
            *cache = vec![0; TEXT_SLOTS];
        }
        let mut h = crate::hashing::FastHasher::new();
        std::hash::Hasher::write(&mut h, s.as_bytes());
        let at = std::hash::Hasher::finish(&h) as usize % TEXT_SLOTS;
        let cached = cache[at];
        if cached != 0 && with_str(slots(cached)[0] as i64, |t| t == s) {
            retain(cached);
            return cached as u64;
        }
        release(cached);
        let made = self.variant(i, new_text(s, ty) as u64) as i64;
        let cache = if i == NUMBER {
            &mut self.numbers
        } else {
            &mut self.texts
        };
        cache[at] = made;
        retain(made);
        made as u64
    }

    /// Variant number `i` (of `VARIANTS`) holding `slot`.
    fn variant(&self, i: usize, slot: u64) -> u64 {
        let p = alloc_zeroed(self.json, self.tags[i], 1);
        slots_mut(p)[0] = slot;
        p as u64
    }
}

impl Build for Builder {
    type Value = u64;
    type Key = i64;

    fn null(&mut self) -> u64 {
        if self.null == 0 {
            self.null = alloc_zeroed(self.json, self.tags[0], 0);
        }
        retain(self.null);
        self.null as u64
    }

    fn bool(&mut self, b: bool) -> u64 {
        if self.bools[b as usize] == 0 {
            self.bools[b as usize] = self.variant(1, b as u64) as i64;
        }
        retain(self.bools[b as usize]);
        self.bools[b as usize] as u64
    }

    fn int(&mut self, n: i64) -> u64 {
        let Some(i) = usize::try_from(n).ok().filter(|i| *i < SMALL_INTS) else {
            return self.number(&n.to_string());
        };
        if self.ints.is_empty() {
            self.ints = vec![0; SMALL_INTS];
        }
        if self.ints[i] == 0 {
            let digits = new_text(&n.to_string(), self.digits) as u64;
            self.ints[i] = self.variant(NUMBER, digits) as i64;
        }
        retain(self.ints[i]);
        self.ints[i] as u64
    }

    fn float(&mut self, _: f64, text: &str) -> u64 {
        self.number(text)
    }

    fn number(&mut self, text: &str) -> u64 {
        self.shared(NUMBER, text)
    }

    fn text(&mut self, s: &str) -> u64 {
        self.shared(TEXT, s)
    }

    fn key(&mut self, s: &str) -> i64 {
        if let Some(&(k, _)) = self.keys.get(s) {
            retain(k);
            return k;
        }
        let k = new_text(s, self.key);
        let hash = hash_slot(k, self.key);
        self.keys.insert(s.into(), (k, hash));
        self.hashes.insert(k, hash);
        retain(k);
        k
    }

    fn array(&mut self, items: std::vec::Drain<'_, u64>) -> u64 {
        let list = alloc_slots(self.list, 0, items.collect());
        self.variant(4, list as u64)
    }

    fn object(&mut self, fields: std::vec::Drain<'_, (i64, u64)>) -> u64 {
        // The parser has rejected duplicate keys, so each one is new.
        let mut table = Table::with_capacity(fields.len());
        for (k, v) in fields {
            let hash = self
                .hashes
                .get(&k)
                .copied()
                .unwrap_or_else(|| hash_slot(k, self.key));
            table.push(hash, k as u64, v);
        }
        let map = alloc(self.map, Data::Map(Box::new(table)));
        self.variant(5, map as u64)
    }

    fn discard(&mut self, value: u64) {
        release(value as i64);
    }

    fn discard_key(&mut self, key: i64) {
        release(key);
    }
}
