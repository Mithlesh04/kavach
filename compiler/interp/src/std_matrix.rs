// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `matrix.product_id`, the product of `std.tensor`'s `a @ b`, in the
//! interpreter (spec 14.33, decisions 0111 and 0136): the lists' items
//! read as the numbers they are, by their first item, and the work done by
//! `kavach_runtime_matrix`, as native code does
//! (`runtime/native/src/rt_matrix.rs`).

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_matrix::{self as mx, Num, Overflow};
use std::rc::Rc;

/// The product's arguments: the two lists and the sizes.
struct Args<'v, 'a> {
    a: &'v [Value<'a>],
    b: &'v [Value<'a>],
    sizes: Vec<usize>,
}

/// The product of numbers of type `T`, read with `from`, given back by
/// `to`.
fn work<'a, T: Num>(
    args: &Args<'_, 'a>,
    from: impl Fn(&Value<'a>) -> T,
    to: impl Fn(T) -> Value<'a>,
) -> Result<Vec<Value<'a>>, Overflow> {
    let a: Vec<T> = args.a.iter().map(&from).collect();
    let b: Vec<T> = args.b.iter().map(&from).collect();
    let size = |i: usize| args.sizes.get(i).copied().unwrap_or(0);
    let out = mx::product(&a, &b, (size(0), size(1), size(2)), size(3) != 0)?;
    Ok(out.into_iter().map(to).collect())
}

fn int<T: TryFrom<i128> + Default>(v: &Value<'_>) -> T {
    match v {
        Value::Int(n, _) => T::try_from(*n).unwrap_or_default(),
        _ => T::default(),
    }
}

impl<'a> Interp<'a> {
    /// `matrix.product_id`, with every argument given.
    pub(crate) fn matrix_call(&mut self, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let empty: Rc<crate::items::Items<'a>> = Rc::default();
        let list = |i: usize| match args.get(i) {
            Some(Value::List(items)) => items.clone(),
            _ => empty.clone(),
        };
        let (a, second) = (list(0), list(1));
        let number = |v: &Value<'a>| match v {
            Value::Int(n, _) => (*n).max(0) as usize,
            _ => 0,
        };
        let sizes: Vec<usize> = list(2).iter().map(number).collect();
        let args = Args {
            a: &a,
            b: &second,
            sizes,
        };
        // An empty list gives an empty list, whatever its items' type.
        let Some(first) = a.first() else {
            return Ok(Value::List(Rc::new(Vec::new().into())));
        };
        let done = match first {
            Value::Float(_) => work(
                &args,
                |v| match v {
                    Value::Float(f) => *f,
                    _ => 0.0,
                },
                Value::Float,
            ),
            Value::Float32(_) => work(
                &args,
                |v| match v {
                    Value::Float32(f) => *f,
                    _ => 0.0,
                },
                Value::Float32,
            ),
            Value::Int(_, p) => {
                let p = *p;
                let back = move |n: i128| Value::Int(n, p);
                match p {
                    Prim::Int8 => work(&args, int::<i8>, move |n| back(n.into())),
                    Prim::Int16 => work(&args, int::<i16>, move |n| back(n.into())),
                    Prim::Int32 => work(&args, int::<i32>, move |n| back(n.into())),
                    Prim::UInt8 => work(&args, int::<u8>, move |n| back(n.into())),
                    Prim::UInt16 => work(&args, int::<u16>, move |n| back(n.into())),
                    Prim::UInt32 => work(&args, int::<u32>, move |n| back(n.into())),
                    Prim::UInt64 => work(&args, int::<u64>, move |n| back(n.into())),
                    _ => work(&args, int::<i64>, move |n| back(n.into())),
                }
            }
            _ => {
                let why = "`@` works on tensors of `Float`, `Float32` or a whole-number type (spec 14.33)";
                return Err(self.panic(why, span));
            }
        };
        match done {
            Ok(items) => Ok(Value::List(Rc::new(items.into()))),
            Err(Overflow) => Err(self.panic(
                "integer overflow in `@`: a sum of products doesn't fit (spec 14.33)",
                span,
            )),
        }
    }
}
