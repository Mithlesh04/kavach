// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `std.matrix` in the interpreter (spec 2.17, decision 0111): the lists'
//! items read as the numbers they are, by their first item, and the work
//! done by `kavach_runtime_matrix`, as native code does
//! (`runtime/native/src/rt_matrix.rs`).

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_matrix::{self as mx, By, Num, Op, Overflow};
use std::rc::Rc;

/// One operation's arguments: the lists, the number to scale by, and the
/// sizes.
struct Args<'v, 'a> {
    a: &'v [Value<'a>],
    b: &'v [Value<'a>],
    x: Option<&'v Value<'a>>,
    sizes: Vec<usize>,
}

/// Does `name` on numbers of type `T`, read with `from`, given back by
/// `to`.
fn work<'a, T: Num>(
    name: &str,
    args: &Args<'_, 'a>,
    from: impl Fn(&Value<'a>) -> T,
    to: impl Fn(T) -> Value<'a>,
) -> Result<Vec<Value<'a>>, Overflow> {
    let a: Vec<T> = args.a.iter().map(&from).collect();
    let b: Vec<T> = args.b.iter().map(&from).collect();
    let size = |i: usize| args.sizes.get(i).copied().unwrap_or(0);
    let out = match name {
        "product_id" => mx::product(&a, &b, (size(0), size(1), size(2)), size(3) != 0)?,
        "zip_id" => mx::zip([Op::Add, Op::Sub, Op::Mul][size(0).min(2)], &a, &b)?,
        "scale_id" => mx::scale(&a, args.x.map(&from).unwrap_or_default())?,
        "negate_id" => mx::negate(&a)?,
        "transpose_id" => mx::transpose(&a, size(0), size(1)),
        _ => mx::sums(
            &a,
            size(0),
            size(1),
            [By::All, By::Rows, By::Cols][size(2).min(2)],
        )?,
    };
    Ok(out.into_iter().map(to).collect())
}

fn int<T: TryFrom<i128> + Default>(v: &Value<'_>) -> T {
    match v {
        Value::Int(n, _) => T::try_from(*n).unwrap_or_default(),
        _ => T::default(),
    }
}

impl<'a> Interp<'a> {
    /// `matrix.product_id` and the others, with every argument given.
    pub(crate) fn matrix_call(&mut self, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
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
        let sizes: Vec<usize> = match name {
            "product_id" => list(2).iter().map(number).collect(),
            "zip_id" => args.get(2).map(number).into_iter().collect(),
            _ => second.iter().map(number).collect(),
        };
        let b: &[Value<'a>] = if matches!(name, "product_id" | "zip_id") {
            &second
        } else {
            &[]
        };
        let args = Args {
            a: &a,
            b,
            x: if name == "scale_id" {
                args.get(1)
            } else {
                None
            },
            sizes,
        };
        // An empty list gives an empty list, whatever its items' type.
        let Some(first) = a.first() else {
            return Ok(Value::List(Rc::new(Vec::new().into())));
        };
        let done = match first {
            Value::Float(_) => work(
                name,
                &args,
                |v| match v {
                    Value::Float(f) => *f,
                    _ => 0.0,
                },
                Value::Float,
            ),
            Value::Float32(_) => work(
                name,
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
                    Prim::Int8 => work(name, &args, int::<i8>, move |n| back(n.into())),
                    Prim::Int16 => work(name, &args, int::<i16>, move |n| back(n.into())),
                    Prim::Int32 => work(name, &args, int::<i32>, move |n| back(n.into())),
                    Prim::UInt8 => work(name, &args, int::<u8>, move |n| back(n.into())),
                    Prim::UInt16 => work(name, &args, int::<u16>, move |n| back(n.into())),
                    Prim::UInt32 => work(name, &args, int::<u32>, move |n| back(n.into())),
                    Prim::UInt64 => work(name, &args, int::<u64>, move |n| back(n.into())),
                    _ => work(name, &args, int::<i64>, move |n| back(n.into())),
                }
            }
            _ => {
                let why =
                    "a matrix holds numbers: `Float`, `Float32` or a whole-number type (spec 2.17)";
                return Err(self.panic(why, span));
            }
        };
        match done {
            Ok(items) => Ok(Value::List(Rc::new(items.into()))),
            Err(Overflow) => Err(self.panic(
                "integer overflow in a matrix: a result doesn't fit (spec 2.17)",
                span,
            )),
        }
    }
}
