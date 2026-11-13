//! `std.compress` in the interpreter (spec 14.21), from
//! `kavach_runtime_core::compress`, as native code does.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_checker::Prim;
use kavach_runtime_core::compress::{self, Fail};

impl<'a> Interp<'a> {
    /// `compress.encode_bytes(data, format, level)` or
    /// `compress.decode_bytes(data, format, most)`.
    pub(crate) fn compress_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => *n as i64,
            _ => 0,
        };
        let Some(Value::Bytes(data)) = args.first() else {
            return Ok(Value::Unit);
        };
        if name == "encode_bytes" {
            let out = compress::encode(data, int(1), int(2));
            return Ok(Value::Bytes(out.as_slice().into()));
        }
        let most = int(2);
        match compress::decode(data, int(1), most.max(0) as usize) {
            Ok(out) => Ok(Value::Bytes(out.as_slice().into())),
            Err(e) => {
                let (variant, fields) = match e {
                    Fail::TooLarge => (
                        "TooLarge",
                        vec![("most", Value::Int(most as i128, Prim::Int))],
                    ),
                    Fail::Corrupt(why) => ("Corrupt", vec![("reason", self.str_value(&why))]),
                };
                Err(Flow::Fail(self.builtin_variant(
                    "CompressError",
                    variant,
                    fields,
                )))
            }
        }
    }
}
