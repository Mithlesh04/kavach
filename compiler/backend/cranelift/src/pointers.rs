// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Reads and writes through a `Ptr` (spec 6.4): one load or store at the
//! address KIR worked out (`compiler/kir/src/lower_ptr.rs`), with no
//! checks, as in C.
//!
//! A volatile one (6.5) is an atomic load or store: Cranelift has no
//! volatile flag, and its alias analysis merges plain loads of one
//! address, but never an atomic one, nor moves memory accesses across
//! it (decision 0145). A float goes through an integer of its size.

use crate::abi::cl_type;
use crate::codegen::FnGen;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};
use kavach_kir::{Expr, RtOp, Type};

impl FnGen<'_, '_> {
    /// `PtrRead(address)` gives a `ty`; `PtrWrite(address, value)` stores;
    /// their volatile kinds as atomics.
    pub(crate) fn ptr_access(&mut self, op: RtOp, ty: Type, args: &[Expr]) -> Option<Value> {
        let addr = self.expr(&args[0])?;
        let volatile = matches!(op, RtOp::PtrReadVolatile | RtOp::PtrWriteVolatile);
        let flags = MemFlags::new();
        match args.get(1) {
            None if volatile => {
                let t = cl_type(ty)?;
                let bits = match t {
                    types::F64 => types::I64,
                    types::F32 => types::I32,
                    t => t,
                };
                let v = self.b.ins().atomic_load(bits, flags, addr);
                if t == bits {
                    return Some(v);
                }
                Some(self.b.ins().bitcast(t, MemFlags::new(), v))
            }
            None => Some(self.b.ins().load(cl_type(ty)?, flags, addr, 0)),
            Some(v) => {
                let mut value = self.expr(v)?;
                if !volatile {
                    self.b.ins().store(flags, value, addr, 0);
                    return None;
                }
                let bits = match v.ty {
                    Type::F64 => Some(types::I64),
                    Type::F32 => Some(types::I32),
                    _ => None,
                };
                if let Some(bits) = bits {
                    value = self.b.ins().bitcast(bits, MemFlags::new(), value);
                }
                self.b.ins().atomic_store(flags, value, addr);
                None
            }
        }
    }
}
