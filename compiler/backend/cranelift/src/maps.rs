//! `m[k] ?? d` for a map with text keys and plain values, read in place
//! as the LLVM backend does (`compiler/backend/llvm/src/maps.rs`): a key
//! that is an item of a list of text is read where it is, so a short one
//! never becomes an object (Sakshi's S7).

use crate::codegen::FnGen;
use crate::data::plain;
use cranelift_codegen::ir::{Value, types};
use kavach_kir::{Expr, ExprKind, Shape, Type};
use kavach_runtime_native::heap::{LEN, PTR};

impl FnGen<'_, '_> {
    /// Whether `t` is a map with text keys and plain values (numbers,
    /// booleans, characters).
    pub(crate) fn text_keys_plain(&self, t: Type) -> bool {
        let Type::Ref(id) = t else {
            return false;
        };
        match &self.shapes[id as usize] {
            Shape::Map(Type::Ref(k), v) => {
                !v.is_ref() && matches!(self.shapes[*k as usize], Shape::Str)
            }
            _ => false,
        }
    }

    /// `args[0][args[1]] ?? args[2]`, the map borrowed and the key read
    /// where it is.
    pub(crate) fn text_get_or(&mut self, e: &Expr, args: &[Expr]) -> Option<Value> {
        let (map, owned) = self.borrow(&args[0], true)?;
        let zero = self.int(types::I64, 0);
        let slot = match &args[1].kind {
            _ if self.is_text_view(&args[1]) => {
                let addr = self.text_view(&args[1])?;
                let fallback = self.slot_of(&args[2]);
                self.call_rt("kv_map_get_item_or", &[map, addr, zero, fallback])[0]
            }
            // The item stays in its list while the fallback is worked out:
            // only when nothing there can change the list.
            ExprKind::Index(x, i) if self.text_list(x.ty) && plain(i) && plain(&args[2]) => {
                let (list, list_owned) = self.borrow(x, true)?;
                let view = if list_owned {
                    (self.load(list, PTR), self.load(list, LEN))
                } else {
                    self.list_view(x, list)
                };
                let index = self.slot_of(i);
                if list_owned || !self.known_in_range(x, i) {
                    self.check_index(view, index, args[1].line);
                }
                let addr = self.text_addr(view.0, index);
                let fallback = self.slot_of(&args[2]);
                let slot = self.call_rt("kv_map_get_item_or", &[map, addr, zero, fallback])[0];
                if list_owned {
                    self.release(list);
                }
                slot
            }
            _ => {
                let (key, key_owned) = self.borrow(&args[1], true)?;
                let fallback = self.slot_of(&args[2]);
                let slot = self.call_rt("kv_map_get_str_or", &[map, key, fallback])[0];
                if key_owned {
                    self.release(key);
                }
                slot
            }
        };
        if owned {
            self.release(map);
        }
        Some(self.unpack(slot, e.ty))
    }
}
