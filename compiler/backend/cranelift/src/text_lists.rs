//! Lists of text in generated code (`docs/architecture/flat-lists.md`):
//! each item is two words, holding short text itself or a pointer to a
//! text object (`runtime/native/src/text_items.rs`). Items go in and out
//! through the runtime; only the length of short ASCII text is read here,
//! from the item's tag.

use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, Value, types};
use kavach_kir::{Expr, ExprKind, RtOp, Type};
use kavach_runtime_native::heap::{CAP, LEN, PTR, RC};
use kavach_runtime_native::text_items::{ASCII, INLINE, LEN_MASK, WIDTH};

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
    pub(crate) fn text_push(&mut self, list: Value, text: Value, ty: Type, line: u32) -> Value {
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
        self.call_rt("kv_text_store", &[addr, text]);
        let len = self.b.ins().iadd_imm(len, 1);
        self.store(len, list, LEN);
        self.b.ins().jump(merge, &[list.into()]);

        self.b.switch_to_block(rare);
        let zero = self.int(types::I64, 0);
        let grown = self.rt_slots(RtOp::ListPush, list, text, zero, ty, line);
        self.b.ins().jump(merge, &[grown.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }
}
