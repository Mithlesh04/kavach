//! Making small objects in generated code: a record, tuple or union value
//! with at most `MAX_SLOTS` fields is taken from the runtime's object pool
//! (`runtime/native/src/pool.rs`) without a call: pop the free list for its
//! size, or cut it from the current block. Only when the block runs out
//! does the runtime make the object.

use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use kavach_kir::{Shape, Type};
use kavach_runtime_native::heap::{FIELDS, FIXED, RC, TAG, TYPE_ID};
use kavach_runtime_native::pool::{MAX_SLOTS, POOL_END, POOL_NEXT};

impl FnGen<'_, '_> {
    /// A new object of type `ty` (variant `tag`) with `n` fields, its
    /// header written and its fields not yet filled; `None` when `ty` is
    /// not such an object (a list) or has too many fields.
    pub(crate) fn alloc_fixed(&mut self, ty: Type, tag: u32, n: usize) -> Option<Value> {
        let Type::Ref(id) = ty else { return None };
        let fixed = matches!(
            self.shapes[id as usize],
            Shape::Record { .. } | Shape::Tuple(_) | Shape::Union { .. } | Shape::Closure(_)
        );
        if !fixed || n > MAX_SLOTS {
            return None;
        }
        let flags = MemFlags::trusted();
        let size = FIELDS as i64 + 8 * n as i64;
        let gv = self.module.declare_data_in_func(self.pool, self.b.func);
        let pool = self.b.ins().symbol_value(types::I64, gv);
        let (pop, check, bump, slow, init, done) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.append_block_param(init, types::I64);
        self.b.append_block_param(done, types::I64);
        let free_at = 8 * n as i32;
        let first = self.b.ins().load(types::I64, flags, pool, free_at);
        self.b.ins().brif(first, pop, &[], check, &[]);

        // A freed object of this size: its first word is the next one.
        self.b.switch_to_block(pop);
        let next = self.b.ins().load(types::I64, flags, first, 0);
        self.b.ins().store(flags, next, pool, free_at);
        self.b.ins().jump(init, &[first.into()]);

        // Room left in the current block?
        self.b.switch_to_block(check);
        let next = self.b.ins().load(types::I64, flags, pool, POOL_NEXT);
        let end = self.b.ins().load(types::I64, flags, pool, POOL_END);
        let room = self.b.ins().isub(end, next);
        let fits = self
            .b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, room, size);
        self.b.ins().brif(fits, bump, &[], slow, &[]);

        self.b.switch_to_block(bump);
        let after = self.b.ins().iadd_imm(next, size);
        self.b.ins().store(flags, after, pool, POOL_NEXT);
        self.b.ins().jump(init, &[next.into()]);

        // The runtime makes the object (header included) and a new block.
        self.b.switch_to_block(slow);
        self.b.set_cold_block(slow);
        let (code, variant, count) = (
            self.code(ty),
            self.int(types::I64, tag as i64),
            self.int(types::I64, n as i64),
        );
        let made = self.call_rt("kv_record", &[code, variant, count])[0];
        self.b.ins().jump(done, &[made.into()]);

        // The header: count 1; tag, kind and field count; type id.
        self.b.switch_to_block(init);
        let p = self.b.block_params(init)[0];
        let one = self.int(types::I64, 1);
        self.b.ins().store(flags, one, p, RC);
        let word = tag as i64 | (FIXED as i64) << 32 | (n as i64) << 48;
        let word = self.int(types::I64, word);
        self.b.ins().store(flags, word, p, TAG);
        let id = self.int(types::I64, id as i64);
        self.b.ins().store(flags, id, p, TYPE_ID);
        self.count_live();
        self.b.ins().jump(done, &[p.into()]);

        self.b.switch_to_block(done);
        Some(self.b.block_params(done)[0])
    }

    /// One more live object (`kv_live`, for the leak check).
    fn count_live(&mut self) {
        let flags = MemFlags::trusted();
        let gv = self.module.declare_data_in_func(self.live, self.b.func);
        let addr = self.b.ins().symbol_value(types::I64, gv);
        let n = self.b.ins().load(types::I64, flags, addr, 0);
        let n = self.b.ins().iadd_imm(n, 1);
        self.b.ins().store(flags, n, addr, 0);
    }
}
