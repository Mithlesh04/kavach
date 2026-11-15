//! Lists of text in generated code (`docs/architecture/flat-lists.md`):
//! each item is two words, holding short text itself or a pointer to a
//! text object (`runtime/native/src/text_items.rs`). Items go in and out
//! through the runtime; only the length of short ASCII text is read here,
//! from the item's tag.

use crate::codegen::FnGen;
use crate::data::plain;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, Value, types};
use kavach_kir::{Expr, ExprKind, RtOp, Type};
use kavach_runtime_native::heap::{CAP, LEN, PTR, RC};
use kavach_runtime_native::text_items::{ASCII, INLINE, LEN_MASK, WIDTH};

/// What `list.push(text)` pushes: a text object (consumed), or an item's
/// two words, with a reference already taken for long text, and the
/// text's type.
pub(crate) enum Pushed {
    Object(Value),
    Words(Value, Value, Type),
}

impl FnGen<'_, '_> {
    /// Whether `t` is a list of text.
    pub(crate) fn text_list(&self, t: Type) -> bool {
        kavach_kir::text_list(self.shapes, t)
    }

    /// The address of item `index` in the buffer `ptr` of a list of text.
    pub(crate) fn text_addr(&mut self, ptr: Value, index: Value) -> Value {
        self.flat_addr(ptr, index, WIDTH)
    }

    /// The item at `addr` as an owned text object of type `ty`.
    pub(crate) fn text_box(&mut self, addr: Value, ty: Type) -> Value {
        let code = self.code(ty);
        self.call_rt("kv_text_box", &[addr, code])[0]
    }

    /// Whether `e` is a `for` variable that is a view of an item of a
    /// list of text (see `text_views`).
    pub(crate) fn is_text_view(&self, e: &Expr) -> bool {
        matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if self.text_views[l as usize])
    }

    /// The item address held by `e`, a text view (`is_text_view`).
    pub(crate) fn text_view(&mut self, e: &Expr) -> Option<Value> {
        match e.kind {
            ExprKind::Local(l) | ExprKind::Move(l) => Some(self.b.use_var(self.vars[l as usize])),
            _ => None,
        }
    }

    /// The number of characters of the item at `addr`: its tag holds it
    /// for short ASCII text; otherwise the runtime counts.
    pub(crate) fn text_len(&mut self, addr: Value) -> Value {
        let (fast, rare, merge) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(merge, types::I64);
        let high = self.load(addr, 8);
        let tag = self.b.ins().ushr_imm(high, 56);
        let kind = self.b.ins().band_imm(tag, (INLINE | ASCII) as i64);
        let short = self
            .b
            .ins()
            .icmp_imm(IntCC::Equal, kind, (INLINE | ASCII) as i64);
        self.b.ins().brif(short, fast, &[], rare, &[]);

        self.b.switch_to_block(fast);
        let len = self.b.ins().band_imm(tag, LEN_MASK as i64);
        self.b.ins().jump(merge, &[len.into()]);

        self.b.switch_to_block(rare);
        let len = self.call_rt("kv_text_len", &[addr])[0];
        self.b.ins().jump(merge, &[len.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }

    /// `list.push(text)` on a list of text (the text object is consumed):
    /// stored in place when nobody else holds the list and it has room;
    /// otherwise the runtime copies or grows it.
    pub(crate) fn text_push(&mut self, list: Value, text: Pushed, ty: Type, line: u32) -> Value {
        let (check_room, fast, rare, merge) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(merge, types::I64);
        let rc = self.load(list, RC);
        let alone = self.b.ins().icmp_imm(IntCC::Equal, rc, 1);
        self.b.ins().brif(alone, check_room, &[], rare, &[]);

        self.b.switch_to_block(check_room);
        let (len, cap) = (self.load(list, LEN), self.load(list, CAP));
        let room = self.b.ins().icmp(IntCC::UnsignedLessThan, len, cap);
        self.b.ins().brif(room, fast, &[], rare, &[]);

        self.b.switch_to_block(fast);
        let ptr = self.load(list, PTR);
        let addr = self.text_addr(ptr, len);
        match text {
            Pushed::Object(p) => {
                self.call_rt("kv_text_store", &[addr, p]);
            }
            Pushed::Words(low, high, _) => {
                self.store(low, addr, 0);
                self.store(high, addr, 8);
            }
        }
        let len = self.b.ins().iadd_imm(len, 1);
        self.store(len, list, LEN);
        self.b.ins().jump(merge, &[list.into()]);

        self.b.switch_to_block(rare);
        let object = match text {
            Pushed::Object(p) => p,
            Pushed::Words(low, high, text) => {
                let code = self.code(text);
                self.call_rt("kv_text_from_words", &[low, high, code])[0]
            }
        };
        let zero = self.int(types::I64, 0);
        let grown = self.rt_slots(RtOp::ListPush, list, object, zero, ty, line);
        self.b.ins().jump(merge, &[grown.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }

    /// The item `e` gives when it is an item of a list of text, `xs[i]` or
    /// a `for` view: its two words, with one more holder for long text, so
    /// a push copies it with no object made (Sakshi's S7).
    pub(crate) fn pushed_words(&mut self, e: &Expr) -> Option<(Value, Value)> {
        let (addr, list) = if self.is_text_view(e) {
            (self.text_view(e)?, None)
        } else if let ExprKind::Index(x, i) = &e.kind
            && self.text_list(x.ty)
            && plain(i)
        {
            let (list, owned) = self.borrow(x, true)?;
            let view = if owned {
                (self.load(list, PTR), self.load(list, LEN))
            } else {
                self.list_view(x, list)
            };
            let index = self.slot_of(i);
            if owned || !self.known_in_range(x, i) {
                self.check_index(view, index, e.line);
            }
            (self.text_addr(view.0, index), owned.then_some(list))
        } else {
            return None;
        };
        let (low, high) = (self.load(addr, 0), self.load(addr, 8));
        let (long_text, done) = (self.b.create_block(), self.b.create_block());
        let tag = self.b.ins().ushr_imm(high, 56);
        let inline = self.b.ins().band_imm(tag, INLINE as i64);
        let long = self.b.ins().icmp_imm(IntCC::Equal, inline, 0);
        self.b.ins().brif(long, long_text, &[], done, &[]);
        self.b.switch_to_block(long_text);
        self.retain(low);
        self.b.ins().jump(done, &[]);
        self.b.switch_to_block(done);
        if let Some(list) = list {
            self.release(list);
        }
        Some((low, high))
    }

    /// Whether `pushed_words` reads `e` where it is.
    pub(crate) fn words_source(&self, e: &Expr) -> bool {
        self.is_text_view(e)
            || matches!(&e.kind, ExprKind::Index(x, i) if self.text_list(x.ty) && plain(i))
    }

    /// `list[index] = item`, the item as its two words (`pushed_words`):
    /// written in place, the old item let go, when nobody else holds the
    /// list; otherwise the runtime copies it.
    pub(crate) fn text_set_words(
        &mut self,
        list: Value,
        index: Value,
        (low, high, text): (Value, Value, Type),
        ty: Type,
        line: u32,
    ) -> Value {
        let (fast, old_long, store, rare, merge) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(merge, types::I64);
        let rc = self.load(list, RC);
        let alone = self.b.ins().icmp_imm(IntCC::Equal, rc, 1);
        self.b.ins().brif(alone, fast, &[], rare, &[]);

        self.b.switch_to_block(fast);
        let view = (self.load(list, PTR), self.load(list, LEN));
        self.check_index(view, index, line);
        let addr = self.text_addr(view.0, index);
        let (old, old_high) = (self.load(addr, 0), self.load(addr, 8));
        let tag = self.b.ins().ushr_imm(old_high, 56);
        let inline = self.b.ins().band_imm(tag, INLINE as i64);
        let long = self.b.ins().icmp_imm(IntCC::Equal, inline, 0);
        self.b.ins().brif(long, old_long, &[], store, &[]);
        self.b.switch_to_block(old_long);
        self.release(old);
        self.b.ins().jump(store, &[]);
        self.b.switch_to_block(store);
        self.store(low, addr, 0);
        self.store(high, addr, 8);
        self.b.ins().jump(merge, &[list.into()]);

        self.b.switch_to_block(rare);
        let code = self.code(text);
        let object = self.call_rt("kv_text_from_words", &[low, high, code])[0];
        let changed = self.rt_slots(RtOp::ListSet, list, index, object, ty, line);
        self.b.ins().jump(merge, &[changed.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }
}
