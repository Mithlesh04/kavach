//! Flat lists in generated code (`docs/architecture/flat-lists.md`): a
//! list of plain records or tuples keeps their fields in its buffer,
//! `width` words per item. Reading `points[i].x` is one load; an item is
//! made into an object (boxed) only when it is used as a whole value.
//!
//! A *view* of an item is the address of its fields minus `FIELDS`, so
//! the usual field reads (`field_value`) work on it unchanged. A view is
//! only ever borrowed: it is never retained, released or stored.

use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use kavach_kir::{Expr, ExprKind, RtOp, Shape, Type};
use kavach_runtime_native::heap::{CAP, FIELDS, LEN, PTR, RC};

fn flags() -> MemFlags {
    MemFlags::trusted()
}

impl FnGen<'_, '_> {
    /// The words per item of list type `t` when it is flat, or 0.
    pub(crate) fn flat_width(&self, t: Type) -> usize {
        kavach_kir::flat_width(self.shapes, t)
    }

    /// The address of item `index` in the buffer `ptr` of a flat list.
    pub(crate) fn flat_addr(&mut self, ptr: Value, index: Value, width: usize) -> Value {
        let offset = self.b.ins().imul_imm(index, 8 * width as i64);
        self.b.ins().iadd(ptr, offset)
    }

    /// A view of the item at `addr` (see the top of this file).
    pub(crate) fn flat_view(&mut self, addr: Value) -> Value {
        self.b.ins().iadd_imm(addr, -(FIELDS as i64))
    }

    /// A new object of type `item` with the `width` fields at `addr`.
    pub(crate) fn flat_box(&mut self, addr: Value, item: Type, width: usize) -> Value {
        let obj = self.alloc_fixed(item, 0, width).expect("a plain record");
        for k in 0..width {
            let word = self.b.ins().load(types::I64, flags(), addr, 8 * k as i32);
            self.init_field(obj, k, word);
        }
        obj
    }

    /// The fields of `e`, a record or tuple about to go into a flat list:
    /// evaluated one by one when `e` builds it there, so no object is made.
    pub(crate) fn flat_fields(&mut self, e: &Expr) -> Option<Vec<Value>> {
        match &e.kind {
            ExprKind::Record { fields, .. } | ExprKind::Tuple(fields) => {
                Some(fields.iter().map(|f| self.slot_of(f)).collect())
            }
            // A block ending in the record (a lambda's body, an inlined
            // call): its statements, then the record's fields.
            ExprKind::Block(b) if b.value.as_deref().is_some_and(builds_fixed) => {
                for s in &b.stmts {
                    self.stmt(s);
                }
                self.flat_fields(b.value.as_deref()?)
            }
            _ => None,
        }
    }

    /// `e` as an item going into a flat list.
    pub(crate) fn flat_item(&mut self, e: &Expr) -> FlatItem {
        match self.flat_fields(e) {
            Some(fields) => FlatItem::Fields(fields),
            None => FlatItem::Object(self.slot_of(e)),
        }
    }

    /// Stores an item at `addr`: its `fields`, or the fields of the object
    /// `obj` (whose reference is consumed).
    fn flat_store(&mut self, addr: Value, item: &FlatItem, width: usize) {
        match item {
            FlatItem::Fields(fields) => {
                for (k, f) in fields.iter().enumerate() {
                    self.b.ins().store(flags(), *f, addr, 8 * k as i32);
                }
            }
            FlatItem::Object(obj) => {
                for k in 0..width {
                    let word = self.field(*obj, k as u32);
                    self.b.ins().store(flags(), word, addr, 8 * k as i32);
                }
                self.release(*obj);
            }
        }
    }

    /// The item as one object, for the runtime's rare paths.
    fn flat_object(&mut self, item: &FlatItem, list_ty: Type) -> Value {
        match item {
            FlatItem::Object(obj) => *obj,
            FlatItem::Fields(fields) => {
                let item_ty = self.item_type(list_ty);
                let obj = self
                    .alloc_fixed(item_ty, 0, fields.len())
                    .expect("a plain record");
                for (k, f) in fields.iter().enumerate() {
                    self.init_field(obj, k, *f);
                }
                obj
            }
        }
    }

    /// The item type of list type `t`.
    pub(crate) fn item_type(&self, t: Type) -> Type {
        match t {
            Type::Ref(id) => match self.shapes[id as usize] {
                Shape::List(item) => item,
                _ => Type::I64,
            },
            _ => Type::I64,
        }
    }

    /// `list.push(item)` on a flat list: written in place when nobody else
    /// holds the list and it has room; otherwise the runtime grows it.
    pub(crate) fn flat_push(&mut self, list: Value, item: FlatItem, ty: Type, line: u32) -> Value {
        let width = self.flat_width(ty);
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
        let addr = self.flat_addr(ptr, len, width);
        self.flat_store(addr, &item, width);
        let len = self.b.ins().iadd_imm(len, 1);
        self.store(len, list, LEN);
        self.b.ins().jump(merge, &[list.into()]);

        self.b.switch_to_block(rare);
        let obj = self.flat_object(&item, ty);
        let zero = self.int(types::I64, 0);
        let grown = self.rt_slots(RtOp::ListPush, list, obj, zero, ty, line);
        self.b.ins().jump(merge, &[grown.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }

    /// `list[i] = item` on a flat list: written in place when nobody else
    /// holds the list and `i` is in range; otherwise the runtime copies the
    /// list or panics.
    pub(crate) fn flat_set(
        &mut self,
        list: Value,
        index: Value,
        obj: Value,
        ty: Type,
        line: u32,
    ) -> Value {
        let width = self.flat_width(ty);
        let (check, fast, rare, merge) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(merge, types::I64);
        let rc = self.load(list, RC);
        let alone = self.b.ins().icmp_imm(IntCC::Equal, rc, 1);
        self.b.ins().brif(alone, check, &[], rare, &[]);

        self.b.switch_to_block(check);
        let len = self.load(list, LEN);
        let inside = self.b.ins().icmp(IntCC::UnsignedLessThan, index, len);
        self.b.ins().brif(inside, fast, &[], rare, &[]);

        self.b.switch_to_block(fast);
        let ptr = self.load(list, PTR);
        let addr = self.flat_addr(ptr, index, width);
        self.flat_store(addr, &FlatItem::Object(obj), width);
        self.b.ins().jump(merge, &[list.into()]);

        self.b.switch_to_block(rare);
        let changed = self.rt_slots(RtOp::ListSet, list, index, obj, ty, line);
        self.b.ins().jump(merge, &[changed.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }

    /// Fills item `i` of a new flat list `list` with `e`.
    pub(crate) fn flat_init(&mut self, list: Value, i: usize, e: &Expr, width: usize) {
        let item = self.flat_item(e);
        let ptr = self.load(list, PTR);
        let addr = self.b.ins().iadd_imm(ptr, (8 * width * i) as i64);
        self.flat_store(addr, &item, width);
    }
}

/// How `body` uses local `var`: (uses that take or replace the whole
/// value, uses other than reading a field or the length). Reading a field
/// (`var.x`) or the length of text neither takes nor needs the value, even
/// as its last use (a move).
pub(crate) fn uses_of(body: &kavach_kir::Block, var: kavach_kir::Local) -> (isize, isize) {
    let is_var = |e: &Expr| matches!(e.kind, ExprKind::Local(l) | ExprKind::Move(l) if l == var);
    // Counted in any order (a field read may be seen before its base).
    let (mut moves, mut uses) = (0, 0);
    kavach_kir::walk::any_in_block(body, &mut |e| {
        match &e.kind {
            ExprKind::Move(l) | ExprKind::Assign(l, _) if *l == var => {
                moves += 1;
                uses += 1;
            }
            ExprKind::Local(l) if *l == var => uses += 1,
            ExprKind::GetField(base, _) if is_var(base) => {
                uses -= 1;
                if matches!(base.kind, ExprKind::Move(_)) {
                    moves -= 1;
                }
            }
            ExprKind::Rt(RtOp::StrLen, args) if is_var(&args[0]) => {
                uses -= 1;
                if matches!(args[0].kind, ExprKind::Move(_)) {
                    moves -= 1;
                }
            }
            _ => {}
        }
        false
    });
    (moves, uses)
}

/// An item going into a flat list.
pub(crate) enum FlatItem {
    /// Its field values, not yet in any object.
    Fields(Vec<Value>),
    /// An object holding it (owned).
    Object(Value),
}

/// Whether `e` builds a record or tuple, perhaps at the end of blocks.
fn builds_fixed(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Record { .. } | ExprKind::Tuple(_) => true,
        ExprKind::Block(b) => b.value.as_deref().is_some_and(builds_fixed),
        _ => false,
    }
}
