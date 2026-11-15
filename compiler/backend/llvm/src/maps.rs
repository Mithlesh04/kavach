//! Reading a map in place in generated code: `m[k] ?? d` for integer
//! keys, and `m[k]` and `m[k] ?? d` for text keys, with no call through
//! the runtime's generic entry; a text key that is an item of a list of
//! text is read where it is (Sakshi's S7).

use crate::func::FnEmit;
use kavach_kir::{Expr, ExprKind};

impl FnEmit<'_> {
    /// Whether `map_read` reads the map of `e`, a `MapGetOr` or `MapGet`.
    pub(crate) fn reads_map_in_place(&self, e: &Expr, args: &[Expr]) -> bool {
        match e.kind {
            ExprKind::Rt(kavach_kir::RtOp::MapGetOr, _) => {
                !e.ty.is_ref() && (self.int_keys(args[0].ty) || self.text_keys_plain(args[0].ty))
            }
            _ => self.text_keys(args[0].ty),
        }
    }

    /// `e` read in place (`reads_map_in_place` said it can be).
    pub(crate) fn map_read(&mut self, e: &Expr, args: &[Expr]) -> Option<String> {
        let ExprKind::Rt(op, _) = &e.kind else {
            return None;
        };
        match op {
            // `m[k] ?? d` for an integer key and a scalar value: the map is
            // read in place, without the generic entry's dispatch.
            kavach_kir::RtOp::MapGetOr if self.int_keys(args[0].ty) && !e.ty.is_ref() => {
                let (map, owned) = self.borrow(&args[0], true)?;
                let (key, fallback) = (self.slot_of(&args[1]), self.slot_of(&args[2]));
                let args3 = [("i64", map.clone()), ("i64", key), ("i64", fallback)];
                let slot = self.call_rt("kv_map_get_int", "i64", &args3, false);
                if owned {
                    self.release(&map);
                }
                Some(self.unpack(&slot, e.ty))
            }
            // `m[k] ?? d` for a text key and a plain value: the same, read
            // in place with the key borrowed.
            kavach_kir::RtOp::MapGetOr if self.text_keys_plain(args[0].ty) && !e.ty.is_ref() => {
                let (map, owned) = self.borrow(&args[0], true)?;
                let slot = if let Some((addr, copy)) = self.key_item(&args[1]) {
                    let fallback = self.slot_of(&args[2]);
                    let copy = (copy as i64).to_string();
                    let args4 = [
                        ("i64", map.clone()),
                        ("i64", addr),
                        ("i64", copy),
                        ("i64", fallback),
                    ];
                    self.call_rt("kv_map_get_item_or", "i64", &args4, false)
                } else {
                    let (key, key_owned) = self.borrow(&args[1], true)?;
                    let fallback = self.slot_of(&args[2]);
                    let args3 = [
                        ("i64", map.clone()),
                        ("i64", key.clone()),
                        ("i64", fallback),
                    ];
                    let slot = self.call_rt("kv_map_get_str_or", "i64", &args3, false);
                    if key_owned {
                        self.release(&key);
                    }
                    slot
                };
                if owned {
                    self.release(&map);
                }
                Some(self.unpack(&slot, e.ty))
            }
            // `m[k]` for a text key and a value held by reference: the map
            // is read in place, and a constant key needs no text object.
            kavach_kir::RtOp::MapGet if self.text_keys(args[0].ty) => {
                let (map, owned) = self.borrow(&args[0], true)?;
                let slot = if let ExprKind::Str(key) = &args[1].kind {
                    let (ptr, len) = self.m.text(key);
                    let args3 = [("i64", map.clone()), ("i64", ptr), ("i64", len.to_string())];
                    self.call_rt("kv_map_get_text", "i64", &args3, false)
                } else if let Some((addr, copy)) = self.key_item(&args[1]) {
                    let copy = (copy as i64).to_string();
                    let args3 = [("i64", map.clone()), ("i64", addr), ("i64", copy)];
                    self.call_rt("kv_map_get_item", "i64", &args3, false)
                } else {
                    let (key, key_owned) = self.borrow(&args[1], true)?;
                    let args2 = [("i64", map.clone()), ("i64", key.clone())];
                    let slot = self.call_rt("kv_map_get_str", "i64", &args2, false);
                    if key_owned {
                        self.release(&key);
                    }
                    slot
                };
                if owned {
                    self.release(&map);
                }
                Some(slot)
            }
            _ => None,
        }
    }
}
