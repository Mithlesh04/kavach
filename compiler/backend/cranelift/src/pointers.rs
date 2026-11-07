//! Reads and writes through a `Ptr` (spec 6.4): one load or store at the
//! address KIR worked out (`compiler/kir/src/lower_ptr.rs`), with no
//! checks, as in C.

use crate::abi::cl_type;
use crate::codegen::FnGen;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value};
use kavach_kir::{Expr, Type};

impl FnGen<'_, '_> {
    /// `PtrRead(address)` gives a `ty`; `PtrWrite(address, value)` stores.
    pub(crate) fn ptr_access(&mut self, ty: Type, args: &[Expr]) -> Option<Value> {
        let addr = self.expr(&args[0])?;
        match args.get(1) {
            None => Some(self.b.ins().load(cl_type(ty)?, MemFlags::new(), addr, 0)),
            Some(v) => {
                let value = self.expr(v)?;
                self.b.ins().store(MemFlags::new(), value, addr, 0);
                None
            }
        }
    }
}
