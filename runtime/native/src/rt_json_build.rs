//! Parsing JSON straight into `Json` values (spec 2.12), with no tree in
//! between: the parser (`kavach_runtime_core::json::parse_with`) hands
//! each value to this builder as soon as it has read it. Maps are made at
//! their final size, and values that repeat are shared: `null`, `true`
//! and `false` are one object each, so are short text and short numbers seen
//! recently, and each key (with its hash kept). A number is a `Number`
//! holding its digits (decision 0071). A `Json` can't be
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

/// Text this long or shorter is shared while it stays in one of
/// `TEXT_SLOTS` slots.
const SHORT_TEXT: usize = 16;
const TEXT_SLOTS: usize = 1024;

/// Keys remembered without hashing them: documents repeat a few keys.
const RECENT_KEYS: usize = 64;

/// Short values seen recently, by hash. A slot that has been of use keeps
/// its value through one value that lands on it, so a run of values seen
/// once (ids) can't push out those that repeat (prices).
#[derive(Default)]
struct Recent {
    values: Vec<i64>,
    used: Vec<bool>,
}

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
    /// Short `Text` and `Number` values seen recently.
    texts: Recent,
    numbers: Recent,
    /// Each key text seen so far, and its hash; the builder holds one
    /// reference to each.
    keys: FastMap<Box<str>, (i64, u64)>,
    /// Keys recently found, by their length and two of their bytes.
    recent_keys: [(i64, u64); RECENT_KEYS],
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
            texts: Recent::default(),
            numbers: Recent::default(),
            keys: FastMap::default(),
            recent_keys: [(0, 0); RECENT_KEYS],
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
        let shared = (self.bools.iter())
            .chain(&self.texts.values)
            .chain(&self.numbers.values);
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
        let recent = if i == NUMBER {
            &mut self.numbers
        } else {
            &mut self.texts
        };
        if recent.values.is_empty() {
            recent.values = vec![0; TEXT_SLOTS];
            recent.used = vec![false; TEXT_SLOTS];
        }
        let mut h = crate::hashing::FastHasher::new();
        std::hash::Hasher::write(&mut h, s.as_bytes());
        let at = std::hash::Hasher::finish(&h) as usize % TEXT_SLOTS;
        let cached = recent.values[at];
        if cached != 0 && with_str(slots(cached)[0] as i64, |t| t == s) {
            recent.used[at] = true;
            retain(cached);
            return cached as u64;
        }
        if std::mem::take(&mut recent.used[at]) {
            return self.variant(i, new_text(s, ty) as u64);
        }
        release(cached);
        let made = self.variant(i, new_text(s, ty) as u64) as i64;
        let recent = if i == NUMBER {
            &mut self.numbers
        } else {
            &mut self.texts
        };
        recent.values[at] = made;
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
    /// The key's text object and its hash.
    type Key = (i64, u64);
    const KEEPS_DIGITS: bool = true;

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
        self.number(&n.to_string())
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

    fn key_id(&self, key: &(i64, u64)) -> Option<u64> {
        // Each key's text has one object (`keys`).
        Some(key.0 as u64)
    }

    fn key(&mut self, s: &str) -> (i64, u64) {
        let b = s.as_bytes();
        let mix = |at: usize| b.get(at).copied().unwrap_or(0) as usize;
        let place = (b.len() + 3 * mix(0) + 5 * mix(b.len().wrapping_sub(1))) % RECENT_KEYS;
        let (k, hash) = self.recent_keys[place];
        if k != 0 && with_str(k, |t| t == s) {
            retain(k);
            return (k, hash);
        }
        let found = match self.keys.get(s) {
            Some(found) => *found,
            None => {
                let k = new_text(s, self.key);
                let made = (k, hash_slot(k, self.key));
                self.keys.insert(s.into(), made);
                made
            }
        };
        self.recent_keys[place] = found;
        retain(found.0);
        found
    }

    fn array(&mut self, items: std::vec::Drain<'_, u64>) -> u64 {
        let list = alloc_slots(self.list, 0, items.collect());
        self.variant(4, list as u64)
    }

    fn object(&mut self, fields: std::vec::Drain<'_, ((i64, u64), u64)>) -> u64 {
        // The parser has rejected duplicate keys, so each one is new.
        let mut table = Table::with_capacity(fields.len());
        for ((k, hash), v) in fields {
            table.push(hash, k as u64, v);
        }
        let map = alloc(self.map, Data::Map(Box::new(table)));
        self.variant(5, map as u64)
    }

    fn discard(&mut self, value: u64) {
        release(value as i64);
    }

    fn discard_key(&mut self, key: (i64, u64)) {
        release(key.0);
    }
}
