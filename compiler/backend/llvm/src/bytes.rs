// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `b[i]` on `Bytes` (spec 2.13) in generated code: the bytes' address
//! and length from two runtime functions that only read, which LLVM may
//! move out of a loop, then a bounds check and one load. A byte scan reads
//! as a list loop does, with no call and no reference count per byte
//! (Sakshi's S5: 591 ms through `BytesAt` for 56 MB).

use crate::func::FnEmit;
use kavach_kir::Expr;

impl FnEmit<'_> {
    pub(crate) fn bytes_at(&mut self, b: &Expr, i: &Expr, line: u32) -> Option<String> {
        let (bytes, owned) = self.borrow(b, crate::heap::plain(i))?;
        let index = self.expr(i)?;
        let data = self.call_rt("kv_bytes_data", "i64", &[("i64", bytes.clone())], false);
        let len = self.call_rt("kv_bytes_count", "i64", &[("i64", bytes.clone())], false);
        let inside = self.val(format!("icmp ult i64 {index}, {len}"));
        let (ok, bad) = (self.label(), self.label());
        self.cond_br(&inside, &ok, &bad);
        self.start(&bad);
        let args = [
            ("i64", index.clone()),
            ("i64", len.clone()),
            ("i32", line.to_string()),
        ];
        self.call_rt("kv_bytes_bad_index", "void", &args, true);
        self.start(&ok);
        let at = self.val(format!("add i64 {data}, {index}"));
        let ptr = self.val(format!("inttoptr i64 {at} to ptr"));
        // A `UInt8`, as this backend holds one.
        let byte = self.val(format!("load i8, ptr {ptr}"));
        if owned {
            self.release(&bytes);
        }
        Some(byte)
    }

    /// `b.len()` on `Bytes`: one runtime function that only reads, which
    /// LLVM may move out of a loop, not the runtime's general `BytesLen`
    /// (Sakshi's S18: an invariant reading `data.len()` cost 27 ns a check).
    pub(crate) fn bytes_len(&mut self, b: &Expr) -> Option<String> {
        let (bytes, owned) = self.borrow(b, true)?;
        let len = self.call_rt("kv_bytes_count", "i64", &[("i64", bytes.clone())], false);
        if owned {
            self.release(&bytes);
        }
        Some(len)
    }
}

#[cfg(test)]
mod tests {
    /// Sakshi's S5: `data[i]` went through the runtime's `BytesAt`, a
    /// call and a reference count each byte. Now it reads the bytes'
    /// address and length through two functions LLVM can hoist out of a
    /// loop, and loads.
    #[test]
    fn a_byte_is_read_where_it_is() {
        let src = "fn count(data: Bytes) -> Int {\n    let n = 0\n    for i in 0..data.len() {\n        if data[i] == 10 {\n            n += 1\n        }\n    }\n    n\n}\n\nfn main() {\n    print(count(\"a\nb\n\".bytes()))\n}\n";
        let c = kavach_driver::compile(src, "main.kav", Default::default());
        let program = kavach_kir::lower(&c).expect("lowers");
        let ir = crate::emit(&program);
        assert!(ir.contains("call i64 @kv_bytes_data("), "{ir}");
        assert!(
            ir.contains(
                "declare i64 @kv_bytes_data(i64) memory(none) nounwind willreturn speculatable"
            ),
            "{ir}"
        );
    }

    /// Sakshi's S18: `data.len()` went through the runtime's general
    /// `BytesLen`; now it is the length function LLVM can hoist.
    #[test]
    fn a_length_of_bytes_is_read_where_it_is() {
        let src = "fn size(data: Bytes) -> Int {
    data.len()
}

fn main() {
    print(size(\"ab\".bytes()))
}
";
        let c = kavach_driver::compile(src, "main.kav", Default::default());
        let program = kavach_kir::lower(&c).expect("lowers");
        let ir = crate::emit(&program);
        assert!(ir.contains("call i64 @kv_bytes_count("), "{ir}");
    }
}
