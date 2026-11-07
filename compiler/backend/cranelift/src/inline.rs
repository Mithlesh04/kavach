//! Heap access written straight into the generated code, without calling
//! the runtime: reference counts, list length, reading items and fields,
//! and pushing onto an unshared list with room left. The runtime is called
//! only on the rare path: freeing an object, growing a list, an index out
//! of range, or an error wrapped by `context`. The offsets are the fixed
//! object layout of `kavach_runtime_native::heap`.

use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use kavach_kir::{Expr, ExprKind, RtOp, Shape, Type};
use kavach_runtime_native::heap::{CAP, FIELDS, FIXED, KIND, LEN, PTR, RC};

fn flags() -> MemFlags {
    MemFlags::trusted()
}

impl FnGen<'_, '_> {
    pub(crate) fn load(&mut self, p: Value, offset: i32) -> Value {
        self.b.ins().load(types::I64, flags(), p, offset)
    }

    pub(crate) fn store(&mut self, v: Value, p: Value, offset: i32) {
        self.b.ins().store(flags(), v, p, offset);
    }

    /// Takes a new reference to `p` (0 means `none`: nothing to count).
    pub(crate) fn retain(&mut self, p: Value) {
        let (count, done) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(p, count, &[], done, &[]);
        self.b.switch_to_block(count);
        let rc = self.load(p, RC);
        let rc = self.b.ins().iadd_imm(rc, 1);
        self.store(rc, p, RC);
        self.b.ins().jump(done, &[]);
        self.b.switch_to_block(done);
    }

    /// Drops a reference to `p`, freeing it when it was the last one.
    pub(crate) fn release(&mut self, p: Value) {
        let (count, free, done) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.ins().brif(p, count, &[], done, &[]);
        self.b.switch_to_block(count);
        let rc = self.load(p, RC);
        let rc = self.b.ins().iadd_imm(rc, -1);
        self.store(rc, p, RC);
        self.b.ins().brif(rc, done, &[], free, &[]);
        self.b.switch_to_block(free);
        self.b.set_cold_block(free);
        self.call_rt("kv_free", &[p]);
        self.b.ins().jump(done, &[]);
        self.b.switch_to_block(done);
    }

    /// Whether `base[i]` is known to be in range (see `unchecked`).
    pub(crate) fn known_in_range(&self, base: &Expr, i: &Expr) -> bool {
        match (&base.kind, &i.kind) {
            (ExprKind::Local(l), ExprKind::Local(x)) => self.unchecked.contains(&(*l, *x)),
            _ => false,
        }
    }

    /// The buffer and length of list `obj`, the value of `base`: read
    /// before the loop when `base` is a steady local (see `steady`).
    pub(crate) fn list_view(&mut self, base: &Expr, obj: Value) -> (Value, Value) {
        if let ExprKind::Local(l) = base.kind
            && let Some((ptr, len)) = self.steady[l as usize]
        {
            return (self.b.use_var(ptr), self.b.use_var(len));
        }
        (self.load(obj, PTR), self.load(obj, LEN))
    }

    /// Item `index` (of type `ty`) of a list, through its `view`, not
    /// retained; panics when the index is out of range.
    pub(crate) fn checked_item(
        &mut self,
        view: (Value, Value),
        index: Value,
        ty: Type,
        line: u32,
    ) -> Value {
        self.check_index(view, index, line);
        self.item_at(view.0, index, ty)
    }

    /// Panics unless `index` is in range of the list whose `view` it is.
    pub(crate) fn check_index(&mut self, view: (Value, Value), index: Value, line: u32) {
        let (fast, rare) = (self.b.create_block(), self.b.create_block());
        let inside = self.b.ins().icmp(IntCC::UnsignedLessThan, index, view.1);
        self.b.ins().brif(inside, fast, &[], rare, &[]);
        self.b.switch_to_block(rare);
        self.b.set_cold_block(rare);
        self.index_panic(view.1, index, line);
        self.b.switch_to_block(fast);
    }

    /// Item `i`, of type `ty`, of a list whose buffer is `ptr`.
    pub(crate) fn item_at(&mut self, ptr: Value, i: Value, ty: Type) -> Value {
        // Packed items (decisions 0030 and 0045), loaded at their own
        // size: `I8`, `U8` and `Bool` are all `I8` here.
        if let size @ 1.. = kavach_kir::packed_size(ty) {
            let offset = self.b.ins().imul_imm(i, size as i64);
            let addr = self.b.ins().iadd(ptr, offset);
            let t = crate::abi::cl_type(ty).expect("a number");
            return self.b.ins().load(t, flags(), addr, 0);
        }
        let offset = self.b.ins().ishl_imm(i, 3);
        let addr = self.b.ins().iadd(ptr, offset);
        self.typed_load(addr, 0, ty)
    }

    /// A slot at `addr + offset` as a value of type `ty`: floats are loaded
    /// straight into a float register.
    fn typed_load(&mut self, addr: Value, offset: i32, ty: Type) -> Value {
        if ty == Type::F64 {
            return self.b.ins().load(types::F64, flags(), addr, offset);
        }
        if ty == Type::F32 {
            // The low 32 bits of the slot (little-endian).
            return self.b.ins().load(types::F32, flags(), addr, offset);
        }
        let slot = self.b.ins().load(types::I64, flags(), addr, offset);
        self.unpack(slot, ty)
    }

    /// Field `i`, of type `ty`, of fixed object `p` (borrowed).
    pub(crate) fn field_value(&mut self, p: Value, i: u32, ty: Type) -> Value {
        self.typed_load(p, FIELDS + 8 * i as i32, ty)
    }

    /// An index out of range of a list with `len` items: a panic. Only a
    /// call and a trap, so the rare path stays small and out of the way.
    fn index_panic(&mut self, len: Value, index: Value, line: u32) {
        let line = self.i32(line as i64);
        self.call_rt("kv_bad_index", &[index, len, line]);
        self.b
            .ins()
            .trap(cranelift_codegen::ir::TrapCode::user(1).expect("a trap code"));
    }

    /// Whether values of type `t` are always fixed objects with their
    /// fields after the header: records, tuples and closures (not unions,
    /// whose errors may be wrapped by `context`, nor `Shared`, a cell
    /// that copies share).
    pub(crate) fn has_fixed_fields(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(
            self.shapes[id as usize],
            Shape::Record { .. } | Shape::Tuple(_) | Shape::Closure(_)
        ))
    }

    fn is_union(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(self.shapes[id as usize], Shape::Union { .. }))
    }

    /// Field `i` of fixed object `p` (borrowed).
    pub(crate) fn field(&mut self, p: Value, i: u32) -> Value {
        self.load(p, FIELDS + 8 * i as i32)
    }

    /// Stores `v` into field `i` of a new fixed object `p`.
    pub(crate) fn init_field(&mut self, p: Value, i: usize, v: Value) {
        self.store(v, p, FIELDS + 8 * i as i32);
    }

    /// Field `i` of `obj` (of type `base`) as an owned value of type
    /// `ty`; `owned` says whether this consumes the caller's reference to
    /// `obj`. A union value may be an error wrapped by `context`: then the
    /// runtime reads through the wrapper.
    pub(crate) fn read_field(
        &mut self,
        obj: Value,
        owned: bool,
        base: Type,
        i: u32,
        ty: Type,
    ) -> Value {
        let (fast, rare, merge) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(merge, types::I64);
        if self.has_fixed_fields(base) {
            self.b.ins().jump(fast, &[]);
        } else if self.is_union(base) {
            let kind = self.b.ins().uload16(types::I64, flags(), obj, KIND);
            let fixed = self.b.ins().icmp_imm(IntCC::Equal, kind, FIXED as i64);
            self.b.ins().brif(fixed, fast, &[], rare, &[]);
        } else {
            self.b.ins().jump(rare, &[]);
        }

        self.b.switch_to_block(fast);
        let slot = self.field(obj, i);
        if ty.is_ref() {
            self.retain(slot);
        }
        if owned {
            self.release(obj);
        }
        self.b.ins().jump(merge, &[slot.into()]);

        self.b.switch_to_block(rare);
        self.b.set_cold_block(rare);
        if !owned {
            // `kv_get` takes a reference of its own.
            self.retain(obj);
        }
        let index = self.int(types::I64, i as i64);
        let slot = self.call_rt("kv_get", &[obj, index])[0];
        self.b.ins().jump(merge, &[slot.into()]);

        self.b.switch_to_block(merge);
        let slot = self.b.block_params(merge)[0];
        self.unpack(slot, ty)
    }

    /// The bytes per item of `t` when it is a list whose items are packed
    /// (decisions 0030 and 0045), otherwise 0.
    pub(crate) fn item_bytes_of(&self, t: Type) -> usize {
        kavach_kir::item_bytes(self.shapes, t)
    }

    /// Stores the low `size` bytes of `slot` as packed item `i` of the
    /// buffer at `ptr`.
    fn store_packed(&mut self, ptr: Value, i: Value, slot: Value, size: usize) {
        let offset = self.b.ins().imul_imm(i, size as i64);
        let addr = self.b.ins().iadd(ptr, offset);
        let t = match size {
            1 => types::I8,
            2 => types::I16,
            _ => types::I32,
        };
        let small = self.b.ins().ireduce(t, slot);
        self.b.ins().store(flags(), small, addr, 0);
    }

    /// The number of items of a list (borrowed).
    pub(crate) fn slot_count(&mut self, p: Value) -> Value {
        self.load(p, LEN)
    }

    /// Item `index` of list `obj` as an owned value of type `ty`; `owned`
    /// says whether this consumes the caller's reference to `obj`. An
    /// index out of range panics.
    pub(crate) fn read_item(
        &mut self,
        obj: Value,
        owned: bool,
        view: (Value, Value),
        index: Value,
        ty: Type,
        line: u32,
    ) -> Value {
        let (fast, rare) = (self.b.create_block(), self.b.create_block());
        let inside = self.b.ins().icmp(IntCC::UnsignedLessThan, index, view.1);
        self.b.ins().brif(inside, fast, &[], rare, &[]);
        self.b.switch_to_block(rare);
        self.b.set_cold_block(rare);
        self.index_panic(view.1, index, line);

        self.b.switch_to_block(fast);
        let item = self.item_at(view.0, index, ty);
        if ty.is_ref() {
            self.retain(item);
        }
        if owned {
            self.release(obj);
        }
        item
    }

    /// `list.len()` on a list: its slot count.
    pub(crate) fn list_len(&mut self, obj: Value, owned: bool) -> Value {
        let n = self.slot_count(obj);
        if owned {
            self.release(obj);
        }
        n
    }

    /// `list.push(v)`: stored in place when nobody else holds the list and
    /// it has room; otherwise the runtime copies or grows it.
    pub(crate) fn list_push(&mut self, list: Value, slot: Value, ty: Type, line: u32) -> Value {
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
        if let size @ 1.. = self.item_bytes_of(ty) {
            self.store_packed(ptr, len, slot, size);
        } else {
            let offset = self.b.ins().ishl_imm(len, 3);
            let addr = self.b.ins().iadd(ptr, offset);
            self.b.ins().store(flags(), slot, addr, 0);
        }
        let len = self.b.ins().iadd_imm(len, 1);
        self.store(len, list, LEN);
        self.b.ins().jump(merge, &[list.into()]);

        self.b.switch_to_block(rare);
        let zero = self.int(types::I64, 0);
        let grown = self.rt_slots(RtOp::ListPush, list, slot, zero, ty, line);
        self.b.ins().jump(merge, &[grown.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }

    /// `list[i] = v` on an owned list: stored in place when nobody else
    /// holds the list and `i` is in range (the old item is released);
    /// otherwise the runtime copies the list or panics.
    pub(crate) fn list_set(
        &mut self,
        list: Value,
        index: Value,
        item: Value,
        ty: Type,
        line: u32,
    ) -> Value {
        let item_ty = match ty {
            Type::Ref(id) => match self.shapes[id as usize] {
                Shape::List(t) => t,
                _ => Type::I64,
            },
            _ => Type::I64,
        };
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
        if let size @ 1.. = kavach_kir::packed_size(item_ty) {
            self.store_packed(ptr, index, item, size);
        } else {
            let offset = self.b.ins().ishl_imm(index, 3);
            let addr = self.b.ins().iadd(ptr, offset);
            let old = self.b.ins().load(types::I64, flags(), addr, 0);
            self.b.ins().store(flags(), item, addr, 0);
            if item_ty.is_ref() {
                self.release(old);
            }
        }
        self.b.ins().jump(merge, &[list.into()]);

        self.b.switch_to_block(rare);
        let changed = self.rt_slots(RtOp::ListSet, list, index, item, ty, line);
        self.b.ins().jump(merge, &[changed.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }

    /// `record.field = v` (field `i`, of type `field_ty`) on an owned
    /// record: stored in place when nobody else holds it, otherwise the
    /// runtime changes a copy.
    pub(crate) fn set_field(&mut self, obj: Value, i: u32, slot: Value, field_ty: Type) -> Value {
        let (fast, rare, merge) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(merge, types::I64);
        let rc = self.load(obj, RC);
        let alone = self.b.ins().icmp_imm(IntCC::Equal, rc, 1);
        self.b.ins().brif(alone, fast, &[], rare, &[]);

        self.b.switch_to_block(fast);
        let old = self.field(obj, i);
        self.init_field(obj, i as usize, slot);
        if field_ty.is_ref() {
            self.release(old);
        }
        self.b.ins().jump(merge, &[obj.into()]);

        self.b.switch_to_block(rare);
        let index = self.int(types::I64, i as i64);
        let changed = self.call_rt("kv_set", &[obj, index, slot])[0];
        self.b.ins().jump(merge, &[changed.into()]);

        self.b.switch_to_block(merge);
        self.b.block_params(merge)[0]
    }

    /// Whether `t` is a map type (not a set).
    pub(crate) fn is_map(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(self.shapes[id as usize], Shape::Map(..)))
    }

    /// Whether `t` is a list type.
    pub(crate) fn is_list(&self, t: Type) -> bool {
        matches!(t, Type::Ref(id) if matches!(self.shapes[id as usize], Shape::List(_)))
    }
}
