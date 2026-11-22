// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `m[xs[i]] = v` with the key an item of a list of text (Sakshi's S8):
//! the key is read where it is. A key already in the map gets its new
//! value with no text object made for the key, and a new key becomes one
//! only when it is added.

use crate::heap::{Data, new_text, release_slot, unique};
use crate::rt_maps::key_value_types;

/// `map[item] = value`: `map` and `value` owned, the key the item at
/// `addr`, which `owned` says is a copy whose long text is let go after.
/// Gives the map.
#[unsafe(no_mangle)]
pub extern "C-unwind" fn kv_map_set_item(
    map: i64,
    addr: i64,
    owned: i64,
    value: i64,
    line: i32,
) -> i64 {
    let item = crate::text_items::item_at(addr);
    let (kt, vt) = key_value_types(map);
    let (hash, found) = crate::text_items::with_item(item, |k| {
        let hash = super::text_hash(k);
        let same = |s: u64| crate::heap::with_str(s as i64, |s| s == k);
        let found = match &*crate::heap::data(map).borrow() {
            Data::Map(t) => t.find(hash, same),
            _ => None,
        };
        (hash, found)
    });
    crate::capacity::check_insert(map, || found.is_some(), line);
    let q = unique(map);
    let key = match found {
        Some(_) => 0,
        None => crate::text_items::with_item(item, |k| new_text(k, kt)),
    };
    if owned != 0 {
        crate::text_items::release_all(item);
    }
    if let Data::Map(table) = &mut *crate::heap::data(q).borrow_mut() {
        match found {
            // A copy keeps its entries in order, so `e` is the same there.
            Some(e) => {
                let old = std::mem::replace(&mut table.slots[e * 2 + 1], value as u64);
                release_slot(old as i64, vt);
            }
            None => {
                table.push(hash, key as u64, value as u64);
            }
        }
    }
    q
}
