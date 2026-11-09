//! Lists in generated code: bounds checks, the buffers of lists a loop
//! holds steady, reading and writing items (one byte per byte for the
//! one-byte types, decision 0030), pushes, and prefetching an item a loop
//! will read (`kir/src/prefetch.rs`). Split from `heap.rs`.

use crate::func::FnEmit;
use crate::heap::ITEM;
use crate::module::llvm_type;
use kavach_kir::{Expr, ExprKind, Local, Shape, Type};
use kavach_runtime_native::heap::{CAP, LEN, PTR, RC};

impl FnEmit<'_> {
    /// Panics unless `index` is below `len` (an index out of range).
    pub(crate) fn check_index(&mut self, index: &str, len: &str, line: u32) {
        let inside = self.val(format!("icmp ult i64 {index}, {len}"));
        let (ok, bad) = (self.label(), self.label());
        self.cond_br(&inside, &ok, &bad);
        self.start(&bad);
        let args = [
            ("i64", index.to_string()),
            ("i64", len.to_string()),
            ("i32", line.to_string()),
        ];
        self.call_rt("kv_bad_index", "void", &args, true);
        self.start(&ok);
    }

    /// Whether `base[i]` is known to be in range (see `unchecked`).
    pub(crate) fn known_in_range(&self, base: &Expr, i: &Expr) -> bool {
        match (&base.kind, &i.kind) {
            (ExprKind::Local(l), ExprKind::Local(x)) => self.unchecked.contains(&(*l, *x)),
            _ => false,
        }
    }

    /// The buffer and length of `list`, the value of `base` (borrowed
    /// unless `owned`): read before the loop when `base` is a steady local
    /// (see `steady`).
    pub(crate) fn view(&mut self, base: &Expr, list: &str, owned: bool) -> (String, String) {
        if let (ExprKind::Local(l), false) = (&base.kind, owned)
            && let Some((ptr, len)) = self.steady[*l as usize].clone()
        {
            let ptr = self.val(format!("load i64, ptr {ptr}"));
            let len = self.val(format!("load i64, ptr {len}"));
            return (ptr, len);
        }
        (self.load_at(list, PTR), self.load_at(list, LEN))
    }

    /// Before a loop: reads the buffer and length of each list local that
    /// `body` indexes and never sets. Nothing else can change such a list
    /// meanwhile: a list is changed in place only by the one variable that
    /// holds it (copy-on-write). Returns the locals, for `let_go`.
    pub(crate) fn hold_steady(&mut self, body: &kavach_kir::Block) -> Vec<Local> {
        let mut indexed: Vec<Local> = Vec::new();
        kavach_kir::walk::any_in_block(body, &mut |e| {
            if let ExprKind::Index(base, _) = &e.kind
                && let ExprKind::Local(l) = base.kind
                && !indexed.contains(&l)
            {
                indexed.push(l);
            }
            false
        });
        indexed.retain(|&l| {
            self.plain_list(self.f.locals[l as usize])
                && self.steady[l as usize].is_none()
                && !kavach_kir::walk::sets_local(body, l)
        });
        for &l in &indexed {
            let list = self.load_local(l).expect("a list");
            let (ptr, len) = (self.slot(Type::I64), self.slot(Type::I64));
            let (p, n) = (self.load_at(&list, PTR), self.load_at(&list, LEN));
            self.ins(format!("store i64 {p}, ptr {ptr}"));
            self.ins(format!("store i64 {n}, ptr {len}"));
            self.steady[l as usize] = Some((ptr, len));
        }
        indexed
    }

    /// After the loop of `hold_steady`.
    pub(crate) fn let_go(&mut self, held: Vec<Local>) {
        for l in held {
            self.steady[l as usize] = None;
        }
    }

    /// Item `index` (of type `ty`) of the list whose buffer and length are
    /// `view`, not retained; when `checked`, an index out of range panics.
    /// Asks the memory for item `index` of `list` (`kir/src/prefetch.rs`):
    /// `llvm.prefetch` never faults, so the index needs no check. Lists
    /// whose items are packed or flat are left alone.
    pub(crate) fn prefetch(&mut self, list: &Expr, index: &Expr) {
        if self.flat_width(list.ty) > 0 || self.text_list(list.ty) {
            return;
        }
        let Some((obj, owned)) = self.borrow(list, true) else {
            return;
        };
        let (buffer, _) = self.view(list, &obj, owned);
        let Some(i) = self.expr(index) else { return };
        let base = self.val(format!("inttoptr i64 {buffer} to ptr"));
        let item = match self.item_bytes_of(list.ty) {
            0 => "i64".to_string(),
            size => format!("i{}", 8 * size),
        };
        let at = self.val(format!("getelementptr {item}, ptr {base}, i64 {i}"));
        self.m.declare(
            "llvm.prefetch.p0",
            "void",
            &["ptr", "i32", "i32", "i32"],
            false,
        );
        self.ins(format!(
            "call void @llvm.prefetch.p0(ptr {at}, i32 0, i32 3, i32 1)"
        ));
        if owned {
            self.release(&obj);
        }
    }

    pub(crate) fn item(
        &mut self,
        view: (String, String),
        index: &str,
        ty: Type,
        line: u32,
        checked: bool,
    ) -> String {
        let (buffer, len) = view;
        if checked {
            let inside = self.val(format!("icmp ult i64 {index}, {len}"));
            let (ok, bad) = (self.label(), self.label());
            self.cond_br(&inside, &ok, &bad);
            self.start(&bad);
            let args = [
                ("i64", index.to_string()),
                ("i64", len),
                ("i32", line.to_string()),
            ];
            self.call_rt("kv_bad_index", "void", &args, true);
            self.start(&ok);
        }
        let base = self.val(format!("inttoptr i64 {buffer} to ptr"));
        // Packed items (decisions 0030 and 0045), loaded at their own
        // size.
        if let size @ 1.. = kavach_kir::packed_size(ty) {
            let bits = 8 * size;
            let at = self.val(format!("getelementptr i{bits}, ptr {base}, i64 {index}"));
            if ty == Type::Bool {
                let byte = self.val(format!("load i8, ptr {at}, {ITEM}"));
                return self.typed_byte(&byte, ty);
            }
            return self.val(format!("load {}, ptr {at}, {ITEM}", llvm_type(ty)));
        }
        let at = self.val(format!("getelementptr i64, ptr {base}, i64 {index}"));
        let t = llvm_type(ty);
        if ty == Type::F64 || ty.bits() == 64 || ty.is_ref() {
            return self.val(format!("load {t}, ptr {at}, {ITEM}"));
        }
        // Smaller values sit in the low bits of their 64-bit slot.
        let slot = self.val(format!("load i64, ptr {at}, {ITEM}"));
        if ty == Type::F32 {
            return self.unpack(&slot, ty);
        }
        self.val(format!("trunc i64 {slot} to {t}"))
    }

    /// A loaded byte as a value of a one-byte type.
    pub(crate) fn typed_byte(&mut self, byte: &str, ty: Type) -> String {
        match ty {
            Type::Bool => self.val(format!("trunc i8 {byte} to i1")),
            _ => byte.to_string(),
        }
    }

    /// `list.len()`.
    pub(crate) fn list_len(&mut self, list: &str) -> String {
        self.load_at(list, LEN)
    }

    /// `list.push(slot)` on an owned list: stored in place when nobody else
    /// holds it and it has room; otherwise the runtime copies or grows it.
    /// Gives the list to keep.
    pub(crate) fn list_push(&mut self, list: &str, slot: &str, ty: Type, line: u32) -> String {
        let result = self.slot(ty);
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
        let base = self.val(format!("inttoptr i64 {buffer} to ptr"));
        if let size @ 1.. = self.item_bytes_of(ty) {
            self.store_packed(&base, &len, slot, size);
        } else {
            let at = self.val(format!("getelementptr i64, ptr {base}, i64 {len}"));
            self.ins(format!("store i64 {slot}, ptr {at}, {ITEM}"));
        }
        let more = self.val(format!("add i64 {len}, 1"));
        self.store_at(&more, list, LEN);
        self.ins(format!("store i64 {list}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let grown = self.rt_op(kavach_kir::RtOp::ListPush, list, slot, "0", ty, line);
        self.ins(format!("store i64 {grown}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }

    /// Whether `t` is a list kept as one slot per item (not a flat list
    /// of records or a list of text, which this backend leaves to
    /// Cranelift for now).
    pub(crate) fn plain_list(&self, t: Type) -> bool {
        let shapes = self.shapes;
        matches!(t, Type::Ref(id) if matches!(shapes[id as usize], Shape::List(_)))
            && kavach_kir::flat_width(shapes, t) == 0
            && !kavach_kir::text_list(shapes, t)
    }

    /// Whether `t` is a plain list of one-slot items that aren't
    /// reference-counted (numbers, booleans, characters), so a store
    /// needn't release what it overwrites.
    pub(crate) fn plain_items(&self, t: Type) -> bool {
        let item = match t {
            Type::Ref(id) => match self.shapes[id as usize] {
                Shape::List(item) => item,
                _ => return false,
            },
            _ => return false,
        };
        self.plain_list(t) && !item.is_ref()
    }

    /// `list[index] = item` on an owned list: stored in place when nobody
    /// else holds it; otherwise the runtime copies it first. Panics when
    /// `index` is out of range. Gives the list to keep.
    pub(crate) fn list_set(
        &mut self,
        list: &str,
        index: &str,
        item: &str,
        ty: Type,
        line: u32,
    ) -> String {
        let result = self.slot(ty);
        let (fast, rare, done) = (self.label(), self.label(), self.label());
        let rc = self.load_at(list, RC);
        let alone = self.val(format!("icmp eq i64 {rc}, 1"));
        self.cond_br(&alone, &fast, &rare);
        self.start(&fast);
        let len = self.load_at(list, LEN);
        self.check_index(index, &len, line);
        let buffer = self.load_at(list, PTR);
        let base = self.val(format!("inttoptr i64 {buffer} to ptr"));
        if let size @ 1.. = self.item_bytes_of(ty) {
            self.store_packed(&base, index, item, size);
        } else {
            let at = self.val(format!("getelementptr i64, ptr {base}, i64 {index}"));
            self.ins(format!("store i64 {item}, ptr {at}, {ITEM}"));
        }
        self.ins(format!("store i64 {list}, ptr {result}"));
        self.br(&done);
        self.start(&rare);
        let copied = self.rt_op(kavach_kir::RtOp::ListSet, list, index, item, ty, line);
        self.ins(format!("store i64 {copied}, ptr {result}"));
        self.br(&done);
        self.start(&done);
        self.val(format!("load i64, ptr {result}"))
    }

    /// The bytes per item of `t` when it is a list whose items are packed
    /// (decisions 0030 and 0045), otherwise 0.
    pub(crate) fn item_bytes_of(&self, t: Type) -> usize {
        kavach_kir::item_bytes(self.shapes, t)
    }

    /// Stores the low `size` bytes of `slot` as packed item `i` of the
    /// buffer at pointer `base`.
    pub(crate) fn store_packed(&mut self, base: &str, i: &str, slot: &str, size: usize) {
        let bits = 8 * size;
        let at = self.val(format!("getelementptr i{bits}, ptr {base}, i64 {i}"));
        let small = self.val(format!("trunc i64 {slot} to i{bits}"));
        self.ins(format!("store i{bits} {small}, ptr {at}, {ITEM}"));
    }
}
