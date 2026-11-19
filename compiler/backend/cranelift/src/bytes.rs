// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `b[i]` on `Bytes` (spec 2.13): the bytes' address and length from the
//! runtime, a bounds check and one load, with no reference count and no
//! trip through the runtime's op entry per byte (as `compiler/backend/
//! llvm/src/bytes.rs`, which LLVM also moves out of the loop).

use crate::codegen::FnGen;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use kavach_kir::Expr;

impl FnGen<'_, '_> {
    pub(crate) fn bytes_at(&mut self, b: &Expr, i: &Expr, line: u32) -> Option<Value> {
        let (bytes, owned) = self.borrow(b, crate::data::plain(i))?;
        let index = self.expr(i)?;
        let data = self.call_rt("kv_bytes_data", &[bytes])[0];
        let len = self.call_rt("kv_bytes_count", &[bytes])[0];
        let (fast, rare) = (self.b.create_block(), self.b.create_block());
        let inside = self.b.ins().icmp(IntCC::UnsignedLessThan, index, len);
        self.b.ins().brif(inside, fast, &[], rare, &[]);
        self.b.switch_to_block(rare);
        self.b.set_cold_block(rare);
        let at = self.i32(line as i64);
        self.call_rt("kv_bytes_bad_index", &[index, len, at]);
        self.b
            .ins()
            .trap(cranelift_codegen::ir::TrapCode::user(1).expect("a trap code"));
        self.b.switch_to_block(fast);
        let addr = self.b.ins().iadd(data, index);
        let byte = self.b.ins().load(types::I8, MemFlags::trusted(), addr, 0);
        if owned {
            self.release(bytes);
        }
        Some(byte)
    }
}
