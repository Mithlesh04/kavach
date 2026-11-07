//! Reads and writes through a `Ptr` (spec 6.4): one load or store at the
//! address KIR worked out (`compiler/kir/src/lower_ptr.rs`), with no
//! checks, as in C. A `Bool` is a byte in memory, as C's `bool` is.

use crate::func::FnEmit;
use crate::module::llvm_type;
use kavach_kir::{Expr, Type};

impl FnEmit<'_> {
    /// `PtrRead(address)` gives a `ty`; `PtrWrite(address, value)` stores.
    pub(crate) fn ptr_access(&mut self, ty: Type, args: &[Expr]) -> Option<String> {
        let addr = self.expr(&args[0])?;
        let p = self.val(format!("inttoptr i64 {addr} to ptr"));
        match args.get(1) {
            None if ty == Type::Bool => {
                let byte = self.val(format!("load i8, ptr {p}"));
                Some(self.val(format!("icmp ne i8 {byte}, 0")))
            }
            None => Some(self.val(format!("load {}, ptr {p}", llvm_type(ty)))),
            Some(v) => {
                let value = self.expr(v)?;
                if v.ty == Type::Bool {
                    let byte = self.val(format!("zext i1 {value} to i8"));
                    self.ins(format!("store i8 {byte}, ptr {p}"));
                } else {
                    self.ins(format!("store {} {value}, ptr {p}", llvm_type(v.ty)));
                }
                None
            }
        }
    }
}
