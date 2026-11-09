//! Records and tuples, and flat lists of them
//! (`docs/architecture/flat-lists.md`), as in the Cranelift backend
//! (`cranelift/src/flat.rs`): a record is a fixed object with its fields
//! after the header; a flat list keeps the fields of each item in its
//! buffer, `width` words per item.
//!
//! A *view* of a flat list's item is the address of its fields minus the
//! header size, so field reads work on it unchanged. A view is only ever
//! borrowed: never retained, released or stored.

use crate::func::FnEmit;
use kavach_kir::{Expr, ExprKind, Shape, Type};
use kavach_runtime_native::heap::{CAP, FIELDS, LEN, PTR, RC};

impl FnEmit<'_> {
    /// Whether values of type `t` are fixed objects with their fields
    /// after the header: records and tuples.
    pub(crate) fn fixed(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(
            self.shapes[id as usize],
            Shape::Record { .. } | Shape::Tuple(_)
        ))
    }

    /// The words per item of list type `t` when it is flat, or 0.
    pub(crate) fn flat_width(&self, t: Type) -> usize {
        kavach_kir::flat_width(self.shapes, t)
    }

    /// A new record or tuple of type `ty` with these field slots (owned).
    pub(crate) fn new_fixed(&mut self, ty: Type, fields: &[String]) -> String {
        self.new_record(ty, 0, fields)
    }

    /// A new record of type `ty`, a union's `variant` (0 for others),
    /// with these field slots (owned).
    pub(crate) fn new_record(&mut self, ty: Type, variant: u32, fields: &[String]) -> String {
        let args = [
            ("i64", Self::code(ty)),
            ("i64", variant.to_string()),
            ("i64", fields.len().to_string()),
        ];
        let obj = self.call_rt("kv_record", "i64", &args, false);
        for (i, f) in fields.iter().enumerate() {
            self.store_item_at(f, &obj, FIELDS + 8 * i as i32);
        }
        obj
    }

    /// Field `i` of fixed object `obj` as a slot (borrowed).
    pub(crate) fn field(&mut self, obj: &str, i: u32) -> String {
        self.load_item_at(obj, FIELDS + 8 * i as i32)
    }

    /// `x.f`: field `f` of `x` as an owned value of type `ty`.
    pub(crate) fn get_field(&mut self, x: &Expr, f: u32, ty: Type) -> Option<String> {
        let (obj, owned) = self.borrow(x, true)?;
        let slot = self.field(&obj, f);
        if ty.is_ref() {
            self.retain(&slot);
        }
        if owned {
            self.release(&obj);
        }
        Some(self.unpack(&slot, ty))
    }

    /// `record.field = v` on an owned record (the slot is owned): stored in
    /// place when nobody else holds the record, otherwise the runtime
    /// changes a copy. Gives the record to keep.
    pub(crate) fn set_field(&mut self, obj: &str, i: u32, slot: &str, field_ty: Type) -> String {
        let result = self.slot(Type::I64);
        let (fast, rare, done) = (self.label(), self.label(), self.label());
        let rc = self.load_at(obj, RC);
        let alone = self.val(format!("icmp eq i64 {rc}, 1"));
        self.cond_br(&alone, &fast, &rare);
        self.start(&fast);
        let old = self.field(obj, i);
        self.store_item_at(slot, obj, FIELDS + 8 * i as i32);
        if field_ty.is_ref() {
            self.release(&old);
        }
        self.ins(format!("store i64 {obj}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let args = [
            ("i64", obj.to_string()),
            ("i64", i.to_string()),
            ("i64", slot.to_string()),
        ];
        let changed = self.call_rt("kv_set", "i64", &args, false);
        self.ins(format!("store i64 {changed}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }

    /// The address of item `index` in a flat list's buffer `ptr`.
    pub(crate) fn flat_addr(&mut self, ptr: &str, index: &str, width: usize) -> String {
        let offset = self.val(format!("mul i64 {index}, {}", 8 * width));
        self.val(format!("add i64 {ptr}, {offset}"))
    }

    /// A view of the item at `addr` (see the top of this file).
    pub(crate) fn flat_view(&mut self, addr: &str) -> String {
        self.val(format!("sub i64 {addr}, {FIELDS}"))
    }

    /// A new object of type `item` with the `width` fields at `addr`.
    pub(crate) fn flat_box(&mut self, addr: &str, item: Type, width: usize) -> String {
        let fields: Vec<String> = (0..width)
            .map(|k| self.load_item_at(addr, 8 * k as i32))
            .collect();
        self.new_fixed(item, &fields)
    }

    /// The field slots of `e`, a record or tuple about to go into a flat
    /// list: evaluated one by one when `e` builds it there (no object), or
    /// read from its object, which is then released.
    pub(crate) fn flat_fields(&mut self, e: &Expr, width: usize) -> Vec<String> {
        if let ExprKind::Record { fields, .. } | ExprKind::Tuple(fields) = &e.kind {
            return fields.iter().map(|f| self.slot_of(f)).collect();
        }
        // A block ending in the record (a lambda's body, an inlined call):
        // its statements, then the record's fields, so no object is made.
        if let ExprKind::Block(b) = &e.kind
            && let Some(v) = b.value.as_deref()
        {
            for s in &b.stmts {
                self.stmt(s);
            }
            return self.flat_fields(v, width);
        }
        let obj = self.expr(e).expect("a record");
        let fields = (0..width as u32).map(|k| self.field(&obj, k)).collect();
        self.release(&obj);
        fields
    }

    /// `list.push(item)` on a flat list (owned), with the item's `fields`:
    /// written in place when nobody else holds the list and it has room;
    /// otherwise the runtime grows or copies it. Gives the list to keep.
    pub(crate) fn flat_push(
        &mut self,
        list: &str,
        fields: &[String],
        ty: Type,
        line: u32,
    ) -> String {
        let width = fields.len();
        let result = self.slot(Type::I64);
        let (room_check, fast, rare, done) =
            (self.label(), self.label(), self.label(), self.label());
        let rc = self.load_at(list, RC);
        let alone = self.val(format!("icmp eq i64 {rc}, 1"));
        self.cond_br(&alone, &room_check, &rare);
        self.start(&room_check);
        let (len, cap) = (self.load_at(list, LEN), self.load_at(list, CAP));
        let room = self.val(format!("icmp ult i64 {len}, {cap}"));
        self.cond_br(&room, &fast, &rare);
        self.start(&fast);
        let buffer = self.load_at(list, PTR);
        let addr = self.flat_addr(&buffer, &len, width);
        for (k, f) in fields.iter().enumerate() {
            self.store_item_at(f, &addr, 8 * k as i32);
        }
        let more = self.val(format!("add i64 {len}, 1"));
        self.store_at(&more, list, LEN);
        self.ins(format!("store i64 {list}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let item_ty = self.item_type(ty);
        let item = self.new_fixed(item_ty, fields);
        let grown = self.rt_op(kavach_kir::RtOp::ListPush, list, &item, "0", ty, line);
        self.ins(format!("store i64 {grown}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }

    /// `xs = xs.set(i, item)` on a flat list, its item given field by
    /// field (`flat_fields`), so no object is made for it: written in
    /// place when nobody else holds the list and `i` is in range,
    /// otherwise the runtime copies the list or panics. Gives whether
    /// `value` was such a store.
    pub(crate) fn assign_flat_set(&mut self, l: kavach_kir::Local, value: &Expr) -> bool {
        let ExprKind::Rt(kavach_kir::RtOp::ListSet, args) = &value.kind else {
            return false;
        };
        let width = self.flat_width(value.ty);
        let is_l = matches!(args[0].kind, ExprKind::Local(x) | ExprKind::Move(x) if x == l);
        if width == 0 || !is_l {
            return false;
        }
        // The index and the fields first: they may read the list.
        let index = self.slot_of(&args[1]);
        let fields = self.flat_fields(&args[2], width);
        let list = self.load_local(l).expect("a list");
        let result = self.slot(Type::I64);
        let (in_range, fast, rare, done) = (self.label(), self.label(), self.label(), self.label());
        let rc = self.load_at(&list, RC);
        let alone = self.val(format!("icmp eq i64 {rc}, 1"));
        self.cond_br(&alone, &in_range, &rare);
        self.start(&in_range);
        let len = self.load_at(&list, LEN);
        let inside = self.val(format!("icmp ult i64 {index}, {len}"));
        self.cond_br(&inside, &fast, &rare);
        self.start(&fast);
        let buffer = self.load_at(&list, PTR);
        let addr = self.flat_addr(&buffer, &index, width);
        for (k, f) in fields.iter().enumerate() {
            self.store_item_at(f, &addr, 8 * k as i32);
        }
        self.ins(format!("store i64 {list}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let item = self.new_fixed(self.item_type(value.ty), &fields);
        let changed = self.rt_op(
            kavach_kir::RtOp::ListSet,
            &list,
            &index,
            &item,
            value.ty,
            value.line,
        );
        self.ins(format!("store i64 {changed}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        let changed = self.val(format!("load i64, ptr {result}"));
        self.store_local(l, &changed);
        true
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
}

/// How `body` uses local `var`: (uses that take or replace the whole
/// value, uses other than reading a field). Reading a field (`var.x`)
/// neither takes nor needs the value, even as its last use (a move); as
/// in the Cranelift backend's `flat::uses_of`.
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
            ExprKind::Rt(kavach_kir::RtOp::StrLen, args) if is_var(&args[0]) => {
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
