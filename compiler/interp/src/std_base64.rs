//! `std.base64` in the interpreter (spec 14.20), from
//! `kavach_runtime_core::base64`, as native code does.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_runtime_core::base64::{self, Base64Fail};

impl<'a> Interp<'a> {
    /// `base64.encode(data, url, pad)` or `base64.decode(text, url)`,
    /// with every argument given (defaults filled in by the caller).
    pub(crate) fn base64_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let flag = |i: usize| matches!(args.get(i), Some(Value::Bool(true)));
        match (name, args.first()) {
            ("encode", Some(Value::Bytes(b))) => {
                let pad = !matches!(args.get(2), Some(Value::Bool(false)));
                Ok(self.str_value(&base64::encode(b, flag(1), pad)))
            }
            ("decode", Some(Value::Str(s))) => match base64::decode(s, flag(1)) {
                Ok(b) => Ok(Value::Bytes(b.as_slice().into())),
                Err(e) => {
                    let (variant, fields) = match e {
                        Base64Fail::BadCharacter(at) => (
                            "BadCharacter",
                            vec![("at", Value::Int(at as i128, kavach_checker::Prim::Int))],
                        ),
                        Base64Fail::BadLength => ("BadLength", Vec::new()),
                        Base64Fail::BadPadding => ("BadPadding", Vec::new()),
                        Base64Fail::ExtraBits => ("ExtraBits", Vec::new()),
                    };
                    Err(Flow::Fail(self.builtin_variant(
                        "Base64Error",
                        variant,
                        fields,
                    )))
                }
            },
            _ => Ok(Value::Unit),
        }
    }
}
