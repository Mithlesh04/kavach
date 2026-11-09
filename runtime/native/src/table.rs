//! The hash table behind maps and sets. Entries stay in insertion order
//! (keys and values in turn, spec 2.2); an open-addressing index finds a
//! key's entry in a few probes however big the map is. Each entry's hash is
//! kept, so growing the index never hashes a key again, except for integer
//! keys, whose hash is cheaper to compute again than to store.
//!
//! Hashing uses a fast mixing function with a seed chosen at random when
//! the program starts, so an attacker who controls the keys cannot predict
//! which keys collide.
//!
//! Integer keys start with a cheaper hash that keeps nearby keys nearby,
//! `2 * (key ^ seed)`: keys 0, 1, 2, … land in every other position, so
//! looking them up in order reads memory in order, as C++'s identity hash
//! does. Unlike C++, the table watches its probes: one longer than
//! `LONG_PROBE` (keys that collide, by accident or by attack) switches the
//! table to the strong mixing hash for good. Lookups watch too, so even a
//! run of keys that collide one step at a time is caught the first time
//! it costs anything. See docs/architecture/hash-maps.md.
//!
//! A small table (up to `SMALL` entries) whose keys are not integers keeps
//! no index: comparing its few stored hashes is as fast, and a JSON
//! document with a million small objects saves a million allocations.

use crate::hashing::{K1, mix, seed};
use std::cell::Cell;

/// Tables with keys that are not integers find entries by scanning their
/// hashes up to this many entries, with no index.
const SMALL: usize = 8;

/// The most entries `Table::with_room` makes room for ahead (2^26: an
/// integer index of 2 GB); a bigger map still grows as it fills.
const MAX_ROOM: usize = 1 << 26;

/// A probe longer than this means the cheap integer hash is not working.
const LONG_PROBE: usize = 64;

/// One position of an integer-keyed table's index: the key and its
/// value together, so a lookup costs one memory access. Sixteen bytes,
/// to keep the probed memory small; an empty position holds the
/// `empty()` key, and the entry numbers live in `fast_e`, which lookups
/// never touch. `slots` is read only when iterating.
#[derive(Clone, Copy)]
struct Bucket {
    key: u64,
    value: u64,
}

/// The key that marks an empty bucket: a random value from this run's
/// seed. A program that uses this very key stores it in `Table::aside`
/// instead of a bucket, so every key still works.
fn empty() -> u64 {
    seed() ^ K1
}

#[derive(Clone)]
pub struct Table {
    /// This run's hash seed (`hashing::seed`), kept here so lookups skip
    /// the `OnceLock` behind it.
    seed: u64,
    /// The key that marks an empty bucket (`empty()`), kept for the same
    /// reason.
    none: u64,
    /// Keys and values in turn, in insertion order.
    pub slots: Vec<u64>,
    /// The hash of each entry (empty when the keys are integers).
    hashes: Vec<u64>,
    /// At each position, the entry number plus 1; 0 when empty. The length
    /// is a power of two, at least twice the number of entries. Only
    /// tables whose keys are not integers use it; integer keys use `fast`.
    index: Vec<u32>,
    /// The integer-keyed index: `Bucket`s at hashed positions, a power of
    /// two long, at least twice the number of entries.
    fast: Vec<Bucket>,
    /// The entry number plus 1 for each `fast` position (0 when empty):
    /// a side array, so lookups only pull `fast` into the caches.
    fast_e: Vec<u32>,
    /// The value and entry number of the one key equal to `empty()`,
    /// when the table holds it.
    aside: Option<(u64, u32)>,
    /// The keys are integers, hashed by `int_hash`.
    int_keys: bool,
    /// Integer keys use the strong hash (after a long probe).
    strong: bool,
    /// A lookup saw a long probe: switch to the strong hash.
    long_probe: Cell<bool>,
    /// Entries the program said it will add (`Map.with_capacity`): the
    /// index starts that big, whichever kind of keys come.
    room: usize,
}

impl Default for Table {
    fn default() -> Table {
        Table {
            seed: seed(),
            none: empty(),
            slots: Vec::new(),
            hashes: Vec::new(),
            index: Vec::new(),
            fast: Vec::new(),
            fast_e: Vec::new(),
            aside: None,
            int_keys: false,
            strong: false,
            long_probe: Cell::new(false),
            room: 0,
        }
    }
}

impl Table {
    /// An empty table with room for `n` entries of keys that are not
    /// integers, so adding them never grows it.
    pub fn with_capacity(n: usize) -> Table {
        let index = if n <= SMALL {
            Vec::new()
        } else {
            vec![0; (n * 2).next_power_of_two()]
        };
        Table {
            slots: Vec::with_capacity(n * 2),
            hashes: Vec::with_capacity(n),
            index,
            ..Table::default()
        }
    }

    /// An empty table with room for `n` entries of any kind of key
    /// (`Map.with_capacity`, spec 2.2). The room is made when the first
    /// entry comes, once the table knows its keys; `n` is capped so a
    /// hostile number can't ask for all the memory at once.
    pub fn with_room(n: usize) -> Table {
        let room = n.min(MAX_ROOM);
        Table {
            slots: Vec::with_capacity(room * 2),
            room,
            ..Table::default()
        }
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.slots.len() / 2
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The hash of entry `e`.
    fn hash_of(&self, e: usize) -> u64 {
        if self.int_keys {
            self.int_hash(self.slots[e * 2])
        } else {
            self.hashes[e]
        }
    }

    /// The entry whose hash is `hash` and whose key satisfies `same`.
    pub fn find(&self, hash: u64, mut same: impl FnMut(u64) -> bool) -> Option<usize> {
        if self.index.is_empty() {
            // A small table: no index (see the top of this file).
            return (0..self.hashes.len())
                .find(|e| self.hashes[*e] == hash && same(self.slots[e * 2]));
        }
        let mask = self.index.len() - 1;
        let mut at = hash as usize & mask;
        loop {
            let e = self.index[at] as usize;
            if e == 0 {
                return None;
            }
            if self.hashes[e - 1] == hash && same(self.slots[(e - 1) * 2]) {
                return Some(e - 1);
            }
            at = (at + 1) & mask;
        }
    }

    /// `find`, looking first at entry `hint`: where the same key was in a
    /// table built the same way (JSON objects of one shape).
    pub fn find_from(
        &self,
        hint: usize,
        hash: u64,
        mut same: impl FnMut(u64) -> bool,
    ) -> Option<usize> {
        if self.hashes.get(hint) == Some(&hash) && same(self.slots[hint * 2]) {
            return Some(hint);
        }
        self.find(hash, same)
    }

    /// The hash of an integer key (see the top of this file).
    fn int_hash(&self, key: u64) -> u64 {
        if self.strong {
            mix(key ^ self.seed, K1)
        } else {
            (key ^ self.seed).wrapping_mul(2)
        }
    }

    /// The entry with integer key `key`. Integer keys are equal when their
    /// slots are, so the stored hashes are not read.
    pub fn find_int(&self, key: u64) -> Option<usize> {
        if key == self.none {
            return self.aside.map(|(_, e)| e as usize - 1);
        }
        self.bucket_of(key).map(|at| self.fast_e[at] as usize - 1)
    }

    /// The value for integer key `key`, in one memory access: the bucket
    /// that proves the key is there holds the value too.
    pub fn get_int(&self, key: u64) -> Option<u64> {
        if key == self.none {
            return self.aside.map(|(v, _)| v);
        }
        self.bucket_of(key).map(|at| self.fast[at].value)
    }

    /// Asks the memory for the bucket integer key `key` would be in
    /// (`kir/src/prefetch.rs`); nothing changes.
    #[allow(unsafe_code)]
    pub fn prefetch_int(&self, key: u64) {
        if self.fast.is_empty() {
            return;
        }
        let at = self.int_hash(key) as usize & (self.fast.len() - 1);
        #[cfg(target_arch = "x86_64")]
        // SAFETY: `at` is inside `fast`, and a prefetch only asks the
        // caches for memory: it reads nothing and never faults.
        unsafe {
            use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            _mm_prefetch::<_MM_HINT_T0>(self.fast.as_ptr().add(at) as *const i8);
        }
    }

    /// The position of integer key `key` (never `empty()`), or `None`
    /// when it is absent.
    fn bucket_of(&self, key: u64) -> Option<usize> {
        if self.fast.is_empty() {
            return None;
        }
        let none = self.none;
        let mask = self.fast.len() - 1;
        let mut at = self.int_hash(key) as usize & mask;
        for probe in 0.. {
            let k = self.fast[at].key;
            if k == key || k == none {
                if probe > LONG_PROBE && !self.strong {
                    self.long_probe.set(true);
                }
                return (k == key).then_some(at);
            }
            at = (at + 1) & mask;
        }
        unreachable!("the index always has empty positions")
    }

    /// Whether a lookup asked for the strong hash (see `make_strong`).
    pub fn wants_strong(&self) -> bool {
        self.long_probe.get() && !self.strong
    }

    /// Switches integer keys to the strong hash, rebuilding the index.
    /// Entry numbers and their order stay the same.
    pub fn make_strong(&mut self) {
        self.strong = true;
        self.long_probe.set(false);
        self.rebuild(self.index.len().max(self.fast.len()));
    }

    /// Sets `value` for integer key `key`: replaces the value of the entry
    /// with that key (giving back the old value), or adds an entry. One
    /// probe finds either.
    pub fn insert_int(&mut self, key: u64, value: u64) -> Option<u64> {
        self.int_keys = true;
        if self.wants_strong() {
            self.make_strong();
        }
        // Grow first: growing may switch the hash.
        self.grow_if_full();
        if key == self.none {
            if let Some((v, e)) = self.aside {
                self.aside = Some((value, e));
                let e = e as usize - 1;
                self.slots[e * 2 + 1] = value;
                return Some(v);
            }
            self.aside = Some((value, self.len() as u32 + 1));
            self.slots.push(key);
            self.slots.push(value);
            return None;
        }
        let none = self.none;
        let mask = self.fast.len() - 1;
        let mut at = self.int_hash(key) as usize & mask;
        let mut probe = 0;
        loop {
            let b = self.fast[at];
            if b.key == none {
                break;
            }
            if b.key == key {
                self.fast[at].value = value;
                let e = self.fast_e[at] as usize - 1;
                return Some(std::mem::replace(&mut self.slots[e * 2 + 1], value));
            }
            at = (at + 1) & mask;
            probe += 1;
        }
        self.fast[at] = Bucket { key, value };
        self.fast_e[at] = self.len() as u32 + 1;
        self.slots.push(key);
        self.slots.push(value);
        if probe > LONG_PROBE && !self.strong {
            self.make_strong();
        }
        None
    }

    /// Adds an entry for a key that is not in the table yet, and returns
    /// how far it was placed from its first position. The index doubles
    /// when it is half full, so it stays small enough to cache.
    pub fn push(&mut self, hash: u64, key: u64, value: u64) -> usize {
        let e = self.len();
        let small = !self.int_keys && self.index.is_empty() && e < SMALL;
        if !small {
            self.grow_if_full();
        }
        if !self.int_keys {
            self.hashes.push(hash);
        }
        self.slots.push(key);
        self.slots.push(value);
        if small { 0 } else { self.place(e) }
    }

    /// Removes entry `e`, returning its key and value. Later entries move
    /// up one place, so the index is rebuilt (from the kept hashes).
    pub fn remove(&mut self, e: usize) -> (u64, u64) {
        let (key, value) = (self.slots[e * 2], self.slots[e * 2 + 1]);
        self.slots.drain(e * 2..e * 2 + 2);
        if !self.int_keys {
            self.hashes.remove(e);
        }
        let size = self.index.len().max(self.fast.len());
        if size > 0 {
            self.rebuild(size);
        }
        (key, value)
    }

    fn grow_if_full(&mut self) {
        let have = if self.int_keys {
            self.fast.len()
        } else {
            self.index.len()
        };
        if (self.len() + 1) * 2 > have {
            let wanted = (self.len() + 1).max(self.room);
            let size = (wanted * 2).next_power_of_two().max(8);
            self.rebuild(size);
        }
    }

    /// Puts entry `e` in the index; returns how many positions it probed.
    fn place(&mut self, e: usize) -> usize {
        let mask = self.index.len() - 1;
        let mut at = self.hash_of(e) as usize & mask;
        let mut probe = 0;
        while self.index[at] != 0 {
            at = (at + 1) & mask;
            probe += 1;
        }
        self.index[at] = e as u32 + 1;
        probe
    }

    fn rebuild(&mut self, size: usize) {
        let mut longest = 0;
        let old = self.fast.len();
        if self.int_keys && old > 0 && size == old * 2 && self.index.is_empty() {
            longest = self.double_int().unwrap_or_else(|| self.place_all_int());
        } else if self.int_keys {
            self.index = Vec::new();
            self.fast.resize(
                size,
                Bucket {
                    key: self.none,
                    value: 0,
                },
            );
            self.fast_e.resize(size, 0);
            longest = self.place_all_int();
        } else {
            self.index.clear();
            self.index.resize(size, 0);
            for e in 0..self.len() {
                longest = longest.max(self.place(e));
            }
        }
        if longest > LONG_PROBE && !self.strong && self.int_keys {
            self.make_strong();
        }
    }

    /// Puts entry `e` (an integer key) in a fresh bucket; returns how
    /// many positions it probed.
    fn place_int(&mut self, e: usize) -> usize {
        let (key, value) = (self.slots[e * 2], self.slots[e * 2 + 1]);
        if key == self.none {
            self.aside = Some((value, e as u32 + 1));
            return 0;
        }
        let mask = self.fast.len() - 1;
        let mut at = self.int_hash(key) as usize & mask;
        let none = self.none;
        let mut probe = 0;
        while self.fast[at].key != none {
            at = (at + 1) & mask;
            probe += 1;
        }
        self.fast[at] = Bucket { key, value };
        self.fast_e[at] = e as u32 + 1;
        probe
    }
}

mod grow;

#[cfg(test)]
mod tests;
