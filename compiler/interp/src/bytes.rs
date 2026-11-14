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
        let prim = self.fixed_number(item, span)?;
        let size = size_of(prim);
        if !b.len().is_multiple_of(size) {
            return Ok(Value::None);
        }
        let read = |c: &[u8]| number(prim, word(c, false));
        Ok(Value::List(Rc::new(
            b.chunks_exact(size).map(read).collect(),
        )))
    }

    /// `b.read<T>(at, big_endian: false)` (spec 2.13): the number whose
    /// bytes start at `at`, or `none` when they don't all fit.
    pub(crate) fn bytes_read(
        &mut self,
        b: &[u8],
        item: &kavach_ast::TypeExpr,
        args: &crate::calls::Args<'a>,
        span: Span,
    ) -> Eval<'a> {
        let prim = self.fixed_number(item, span)?;
        let at = match args.first() {
            Some((_, Value::Int(n, _))) => usize::try_from(*n).ok(),
            _ => None,
        };
        let size = size_of(prim);
        Ok(match at.and_then(|at| b.get(at..at.checked_add(size)?)) {
            Some(c) => number(prim, word(c, big_endian(args))),
            None => Value::None,
        })
    }

    /// `Bytes.of<T>(n, big_endian: false)` (spec 2.13): the bytes of `n`.
    pub(crate) fn bytes_of_number(
        &mut self,
        item: &kavach_ast::TypeExpr,
        args: &crate::calls::Args<'a>,
        span: Span,
    ) -> Eval<'a> {
        let prim = self.fixed_number(item, span)?;
        let bits = match args.first().map(|(_, v)| v) {
            Some(Value::Int(n, _)) => *n as u64,
            Some(Value::Float32(f)) => u64::from(f.to_bits()),
            Some(Value::Float(f)) if prim == Prim::Float32 => u64::from((*f as f32).to_bits()),
            Some(Value::Float(f)) => f.to_bits(),
            _ => 0,
        };
        let mut out = bits.to_le_bytes()[..size_of(prim)].to_vec();
        if big_endian(args) {
            out.reverse();
        }
        Ok(Value::Bytes(out.into()))
    }

    /// The number type a `Bytes` method's type argument names.
    fn fixed_number(
        &self,
        item: &kavach_ast::TypeExpr,
        span: Span,
    ) -> Result<Prim, crate::value::Flow<'a>> {
        match self.ty_of(item) {
            Some(kavach_checker::Ty::Prim(p)) => Ok(p),
            _ => Err(self.panic("`Bytes` reads numbers", span)),
        }
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
            "index_of" => {
                let part = match args.first() {
                    Some(Value::Bytes(p)) => p.clone(),
                    _ => Rc::from(&[][..]),
                };
                match kavach_runtime_core::files::find_bytes(b, &part) {
                    Some(i) => Value::Int(i as i128, Prim::Int),
                    None => Value::None,
                }
            }
            _ => return Err(self.panic(format!("`Bytes` has no method `{name}`"), span)),
        })
    }
}

/// How many bytes a number of type `prim` takes (spec 2.13).
fn size_of(prim: Prim) -> usize {
    match prim {
        Prim::Int8 | Prim::UInt8 => 1,
        Prim::Int16 | Prim::UInt16 => 2,
        Prim::Int32 | Prim::UInt32 | Prim::Float32 => 4,
        _ => 8,
    }
}

/// Bytes `c` (at most 8) as the low bytes of a word, in either order.
fn word(c: &[u8], big_endian: bool) -> u64 {
    let mut w = [0; 8];
    w[..c.len()].copy_from_slice(c);
    if big_endian {
        w[..c.len()].reverse();
    }
    u64::from_le_bytes(w)
}

/// The number of type `prim` whose bytes are the low ones of `bits`.
fn number<'a>(prim: Prim, bits: u64) -> Value<'a> {
    match prim {
        Prim::Float => Value::Float(f64::from_bits(bits)),
        Prim::Float32 => Value::Float32(f32::from_bits(bits as u32)),
        Prim::Int8 => Value::Int(i128::from(bits as u8 as i8), prim),
        Prim::Int16 => Value::Int(i128::from(bits as u16 as i16), prim),
        Prim::Int32 => Value::Int(i128::from(bits as u32 as i32), prim),
        Prim::Int => Value::Int(i128::from(bits as i64), prim),
        _ => Value::Int(i128::from(bits), prim),
    }
}

/// Whether `big_endian: true` was given, by name or second.
fn big_endian(args: &crate::calls::Args<'_>) -> bool {
    let given = |(i, (name, _)): &(usize, &(Option<&str>, Value<'_>))| match name {
        Some(n) => *n == "big_endian",
        None => *i == 1,
    };
    args.iter()
        .enumerate()
        .filter(given)
        .any(|(_, (_, v))| matches!(v, Value::Bool(true)))
}
