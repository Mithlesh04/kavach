//! Growing the index of integer keys in place: doubling keeps the old
//! buckets where they are (a big `realloc` moves pages on Linux, copying
//! nothing), so only the new half is fresh memory, and each key moves at
//! most once.

use super::{Bucket, Table};

impl Table {
    /// Doubles the integer index in place; how far the longest key had to
    /// look, or `None` when a key would have to wrap past the end of the
    /// doubled index (then the caller places every key again).
    ///
    /// The bucket of a key in the doubled index is its old one, or that
    /// plus the old size. Runs of full buckets are taken in order, starting
    /// just past an empty one, so a key's way from its bucket crosses only
    /// buckets already taken: holes left by keys moved later can't cut it.
    /// Only a key wrapping from the end of the new half to the start of the
    /// old one could cross keys not taken yet.
    pub(super) fn double_int(&mut self) -> Option<usize> {
        let old = self.fast.len();
        let none = self.none;
        let empty = Bucket {
            key: none,
            value: 0,
        };
        self.fast.resize(old * 2, empty);
        self.fast_e.resize(old * 2, 0);
        let mask = old * 2 - 1;
        // The index is at most half full, so there is an empty bucket.
        let start = (0..old).find(|&i| self.fast[i].key == none)? + 1;
        let mut longest = 0;
        for n in 0..old {
            let at = (start + n) & (old - 1);
            let b = self.fast[at];
            if b.key == none {
                continue;
            }
            let e = self.fast_e[at];
            self.fast[at] = empty;
            self.fast_e[at] = 0;
            let mut to = self.int_hash(b.key) as usize & mask;
            let mut probe = 0;
            while self.fast[to].key != none {
                to = (to + 1) & mask;
                if to == 0 {
                    return None;
                }
                probe += 1;
            }
            self.fast[to] = b;
            self.fast_e[to] = e;
            longest = longest.max(probe);
        }
        Some(longest)
    }

    /// Places every integer key again, in an index of the size it has.
    pub(super) fn place_all_int(&mut self) -> usize {
        let empty = Bucket {
            key: self.none,
            value: 0,
        };
        self.fast.fill(empty);
        self.fast_e.fill(0);
        self.aside = None;
        (0..self.len()).fold(0, |longest, e| longest.max(self.place_int(e)))
    }
}
