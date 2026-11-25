// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Reads and writes through a `Ptr` (spec 6.4): one load or store at the
//! address KIR worked out (`compiler/kir/src/lower_ptr.rs`), with no
//! checks, as in C. A `Bool` is a byte in memory, as C's `bool` is. A
//! volatile one (6.5) is LLVM's `load volatile` or `store volatile`.

use crate::func::FnEmit;
use crate::module::llvm_type;
use kavach_kir::{Expr, RtOp, Type};

impl FnEmit<'_> {
    /// `PtrRead(address)` gives a `ty`; `PtrWrite(address, value)` stores;
    /// their volatile kinds the same, marked volatile.
    pub(crate) fn ptr_access(&mut self, op: RtOp, ty: Type, args: &[Expr]) -> Option<String> {
        let addr = self.expr(&args[0])?;
        let p = self.val(format!("inttoptr i64 {addr} to ptr"));
        let volatile = match op {
            RtOp::PtrReadVolatile | RtOp::PtrWriteVolatile => " volatile",
            _ => "",
        };
        match args.get(1) {
            None if ty == Type::Bool => {
                let byte = self.val(format!("load{volatile} i8, ptr {p}"));
                Some(self.val(format!("icmp ne i8 {byte}, 0")))
            }
            None => Some(self.val(format!("load{volatile} {}, ptr {p}", llvm_type(ty)))),
            Some(v) => {
                let value = self.expr(v)?;
                if v.ty == Type::Bool {
                    let byte = self.val(format!("zext i1 {value} to i8"));
                    self.ins(format!("store{volatile} i8 {byte}, ptr {p}"));
                } else {
                    let t = llvm_type(v.ty);
                    self.ins(format!("store{volatile} {t} {value}, ptr {p}"));
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// Writes 1 then 2 at one address, and reads it twice: `opt -O3`
    /// keeps every volatile access, where it drops plain ones (spec 6.5).
    fn optimized(write: &str, read: &str) -> Option<String> {
        let (one, two) = (write.replace('V', "1"), write.replace('V', "2"));
        let src = format!(
            "fn poke(p: Ptr<UInt32>) -> UInt32 {{\n    unsafe {{\n        {one}\n        {two}\n        {read} + {read}\n    }}\n}}\n\nfn main() {{\n    unsafe {{\n        const p: Ptr<UInt32> = alloc(1)\n        print(poke(p))\n        free(p)\n    }}\n}}\n"
        );
        let options = kavach_driver::Options {
            allow_unsafe: true,
            ..Default::default()
        };
        let c = kavach_driver::compile(&src, "main.kav", options);
        assert!(!c.has_errors(), "{:?}", c.messages);
        let program = kavach_kir::lower(&c).expect("lowers");
        let ir = crate::emit(&program);
        let volatile = write.contains("volatile");
        assert_eq!(ir.contains("store volatile i32"), volatile, "{ir}");
        assert_eq!(ir.contains("load volatile i32"), volatile, "{ir}");
        let Some(bin) = crate::tools() else {
            eprintln!("LLVM's tools aren't installed: only the IR is checked");
            return None;
        };
        let dir = kavach_backend_cranelift::build_folder("kavach-llvm-volatile");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("p.ll"), &ir).unwrap();
        let opt = std::process::Command::new(bin.join("opt"))
            .args(["-O3", "-S"])
            .arg(dir.join("p.ll"))
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            opt.status.success(),
            "{}",
            String::from_utf8_lossy(&opt.stderr)
        );
        Some(String::from_utf8(opt.stdout).unwrap())
    }

    #[test]
    fn volatile_accesses_stay_after_optimizing() {
        let Some(ir) = optimized("p.write_volatile(V)", "p.read_volatile()") else {
            return;
        };
        assert!(ir.contains("store volatile i32 1, ptr"), "{ir}");
        assert!(ir.contains("store volatile i32 2, ptr"), "{ir}");
        assert_eq!(ir.matches("load volatile i32, ptr").count(), 2, "{ir}");
        // The same with plain writes and reads: the first write goes, and
        // the reads are of the 2 just written.
        let Some(plain) = optimized("p.write(0, V)", "p.read(0)") else {
            return;
        };
        assert!(!plain.contains("store i32 1, ptr"), "{plain}");
        assert!(!plain.contains("load i32, ptr"), "{plain}");
    }
}
