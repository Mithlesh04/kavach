//! `Bytes` (spec 2.13): raw data, one byte per byte.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use std::rc::Rc;

fn byte<'a>(b: u8) -> Value<'a> {
    Value::Int(b as i128, Prim::UInt8)
}

/// How `Bytes` prints: its bytes in hexadecimal, `Bytes(6869)`.
pub(crate) fn show(b: &[u8]) -> String {
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("Bytes({hex})")
}

/// The panic for byte `i` of `n` bytes.
pub(crate) fn out_of_range(i: i128, n: usize) -> String {
    kavach_runtime_core::bytes_index_panic(i, n)
}

impl<'a> Interp<'a> {
    /// `Bytes(list)`: bytes from a list of `UInt8`.
    pub(crate) fn bytes_of(&mut self, list: Value<'a>) -> Value<'a> {
        let items = match list {
            Value::List(items) => items,
            _ => Rc::new(Vec::new()),
        };
        let b: Vec<u8> = items
            .iter()
            .map(|v| match v {
                Value::Int(n, _) => *n as u8,
                _ => 0,
            })
            .collect();
        Value::Bytes(b.into())
    }

    /// `b[i]`.
    pub(crate) fn byte_at(&self, b: &[u8], i: i128, span: Span) -> Eval<'a> {
        match usize::try_from(i).ok().and_then(|i| b.get(i)) {
            Some(x) => Ok(byte(*x)),
            None => Err(self.panic(out_of_range(i, b.len()), span)),
        }
    }

    /// `b.as_list<T>()` (spec 2.13): little-endian numbers, or `none`
    /// when `b` isn't a whole number of them.
    pub(crate) fn bytes_as_list(
        &mut self,
        b: &[u8],
        item: &kavach_ast::TypeExpr,
        span: Span,
    ) -> Eval<'a> {
        use kavach_checker::Ty;
        let prim = match self.ty_of(item) {
            Some(Ty::Prim(p)) => p,
            _ => return Err(self.panic("`as_list` reads numbers", span)),
        };
        let size = match prim {
            Prim::Int8 | Prim::UInt8 => 1,
            Prim::Int16 | Prim::UInt16 => 2,
            Prim::Int32 | Prim::UInt32 | Prim::Float32 => 4,
            _ => 8,
        };
        if !b.len().is_multiple_of(size) {
            return Ok(Value::None);
        }
        let number = |c: &[u8]| {
            let mut word = [0; 8];
            word[..size].copy_from_slice(c);
            let bits = u64::from_le_bytes(word);
            match prim {
                Prim::Float => Value::Float(f64::from_bits(bits)),
                Prim::Float32 => Value::Float32(f32::from_bits(bits as u32)),
                Prim::Int8 => Value::Int(i128::from(bits as u8 as i8), prim),
                Prim::Int16 => Value::Int(i128::from(bits as u16 as i16), prim),
                Prim::Int32 => Value::Int(i128::from(bits as u32 as i32), prim),
                Prim::Int => Value::Int(i128::from(bits as i64), prim),
                _ => Value::Int(i128::from(bits), prim),
            }
        };
        Ok(Value::List(Rc::new(
            b.chunks_exact(size).map(number).collect(),
        )))
    }

    pub(crate) fn bytes_method(
        &mut self,
        b: &Rc<[u8]>,
        name: &str,
        args: &[Value<'a>],
        span: Span,
    ) -> Eval<'a> {
        let int = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => *n,
            _ => 0,
        };
        Ok(match name {
            "len" => Value::Int(b.len() as i128, Prim::Int),
            "get" => match usize::try_from(int(0)).ok().and_then(|i| b.get(i)) {
                Some(x) => byte(*x),
                None => Value::None,
            },
            "slice" => {
                let clamp = |n: i128| n.clamp(0, b.len() as i128) as usize;
                let (start, end) = (clamp(int(0)), clamp(int(1)));
                Value::Bytes(b[start..end.max(start)].into())
            }
            "text" => match std::str::from_utf8(b) {
                Ok(s) => self.str_value(s),
                Err(_) => Value::None,
            },
            "to_list" => Value::List(Rc::new(b.iter().map(|x| byte(*x)).collect())),
            "crc32c" => Value::Int(kavach_runtime_core::files::crc32c(b) as i128, Prim::Int),
            "xxhash64" => Value::Int(
                kavach_runtime_core::files::xxhash64(b) as i128,
                Prim::UInt64,
            ),
            "is_empty" => Value::Bool(b.is_empty()),
            _ => return Err(self.panic(format!("`Bytes` has no method `{name}`"), span)),
        })
    }
}
