// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Built-in methods of values that are one run-time operation each
//! (spec 2.2, 2.13, 14.2, 14.10): text, lists, dicts, `Bytes`, sockets,
//! channels, optionals and whole numbers. Split from `lower_methods.rs`,
//! which finds the method and works out its receiver and arguments.

use crate::ir::{self, ExprKind as K, Shape, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::Span;

impl<'a> Lowerer<'a> {
    /// `values` is the receiver, then the arguments, each lowered.
    pub(crate) fn value_method(
        &mut self,
        name: &str,
        mut values: Vec<ir::Expr>,
        ty: Type,
        line: u32,
        span: Span,
    ) -> Lowered<ir::Expr> {
        let shape = self.shapes.shape(values[0].ty).cloned();
        let is_int = values[0].ty.is_int();
        // `b.is_empty()` on `Bytes` is `b.len() == 0`.
        if matches!(shape, Some(Shape::Bytes)) && name == "is_empty" {
            let len = ir::Expr {
                kind: K::Rt(RtOp::BytesLen, values),
                ty: Type::I64,
                line,
            };
            let zero = ir::Expr {
                kind: K::Int(0),
                ty: Type::I64,
                line,
            };
            return Ok(ir::Expr {
                kind: K::Binary(ir::BinOp::Eq, Box::new(len), Box::new(zero)),
                ty: Type::Bool,
                line,
            });
        }
        let op = match (shape, name) {
            (Some(Shape::Str), "len") => RtOp::StrLen,
            (Some(Shape::Str), "is_empty") => RtOp::StrIsEmpty,
            (Some(Shape::Str), "upper") => RtOp::StrUpper,
            (Some(Shape::Str), "lower") => RtOp::StrLower,
            (Some(Shape::Str), "trim") => RtOp::StrTrim,
            (Some(Shape::Str), "contains") => {
                // `s.contains(c)` with a `Char`: its text (spec 14.2).
                let part = values.pop().expect("a part");
                values.push(match part.ty {
                    Type::Char => ir::Expr {
                        kind: K::Format(vec![part]),
                        ty: self.str_type(),
                        line,
                    },
                    _ => part,
                });
                RtOp::StrContains
            }
            (Some(Shape::Str), "starts_with") => RtOp::StrStartsWith,
            (Some(Shape::Str), "ends_with") => RtOp::StrEndsWith,
            (Some(Shape::Str), "repeat") => RtOp::StrRepeat,
            (Some(Shape::List(_)), "enumerate") if values.len() == 1 => {
                let list = values.pop().expect("the list");
                return Ok(self.enumerated(list, ty, line));
            }
            (Some(Shape::List(_)), "len") => RtOp::ListLen,
            (Some(Shape::List(_)), "is_empty") => RtOp::ListIsEmpty,
            (Some(Shape::List(_)), "is_full") => RtOp::ListIsFull,
            // `ring.to_list()` (spec 2.2): a list, without the ring's limit.
            (Some(Shape::List(_)), "to_list") => RtOp::ListNoLimit,
            (Some(Shape::List(item)), "push") => {
                let v = values.pop().expect("an item");
                values.push(self.coerce(v, item)?);
                RtOp::ListPush
            }
            (Some(Shape::List(_)), "get") => RtOp::ListGet,
            (Some(Shape::List(_)), "first") => RtOp::ListFirst,
            (Some(Shape::List(_)), "last") => RtOp::ListLast,
            (Some(Shape::List(_)), "contains") => RtOp::ListContains,
            (Some(Shape::Map(..)), "get") => RtOp::MapGet,
            (Some(Shape::Map(k, _)), "remove") => {
                let key = values.pop().expect("a key");
                values.push(self.coerce(key, k)?);
                RtOp::MapRemove
            }
            (Some(Shape::Map(..)), "contains_key") => RtOp::MapContains,
            (Some(Shape::Map(..)), "len") => RtOp::MapLen,
            (Some(Shape::Map(..)), "is_empty") => RtOp::MapIsEmpty,
            (Some(Shape::Map(..)), "is_full") => RtOp::MapIsFull,
            (Some(Shape::Map(..)), "keys") => RtOp::MapKeys,
            (Some(Shape::Map(..)), "values") => RtOp::MapValues,
            (Some(Shape::Shared(_)), "get") => RtOp::SharedGet,
            (Some(Shape::Shared(_)), "weak") => RtOp::WeakMake,
            (Some(Shape::Shared(_)), "shared") => RtOp::WeakShared,
            (Some(Shape::Channel(t)), "put") => {
                let value = values.pop().expect("a value");
                values.push(self.coerce(value, t)?);
                RtOp::ChanPut
            }
            (Some(Shape::Channel(t)), "try_put") => {
                let value = values.pop().expect("a value");
                values.push(self.coerce(value, t)?);
                RtOp::ChanTryPut
            }
            (Some(Shape::Channel(_)), "close") => RtOp::ChanClose,
            (Some(Shape::Str), "bytes") => RtOp::StrBytes,
            (Some(Shape::Bytes), "len") => RtOp::BytesLen,
            (Some(Shape::Socket), "accept") => RtOp::NetAccept,
            (Some(Shape::Socket), "address") => RtOp::NetAddress,
            (Some(Shape::Socket), "read") => RtOp::NetRead,
            (Some(Shape::Socket), "write") => RtOp::NetWrite,
            (Some(Shape::Socket), "close") => RtOp::NetClose,
            (Some(Shape::Socket), "protocol") => RtOp::NetProtocol,
            // A connection's address is the other side's.
            (Some(Shape::Socket), "peer") => RtOp::NetAddress,
            (Some(Shape::Socket), "client_cert") => RtOp::NetClientCert,
            (Some(Shape::Socket), "peer_certificate") => RtOp::NetPeerCertificate,
            (Some(Shape::Socket), "open_stream") => RtOp::NetOpenStream,
            (Some(Shape::Socket), "read_stream") => RtOp::NetReadStream,
            (Some(Shape::Socket), "write_stream") if values.len() == 4 => {
                // Three operands at most: the stream and `done` in one.
                let done = values.pop().expect("done");
                let data = values.pop().expect("data");
                let stream = values.pop().expect("stream");
                let int = |n: i128| ir::Expr {
                    kind: K::Int(n),
                    ty: Type::I64,
                    line,
                };
                let flag = ir::Expr {
                    kind: K::If(
                        Box::new(done),
                        crate::lower_expr::wrap(int(1), true),
                        crate::lower_expr::wrap(int(0), true),
                    ),
                    ty: Type::I64,
                    line,
                };
                let doubled = ir::Expr {
                    kind: K::Binary(ir::BinOp::Mul, Box::new(stream), Box::new(int(2))),
                    ty: Type::I64,
                    line,
                };
                values.push(ir::Expr {
                    kind: K::Binary(ir::BinOp::Add, Box::new(doubled), Box::new(flag)),
                    ty: Type::I64,
                    line,
                });
                values.push(data);
                RtOp::NetWriteStream
            }
            (Some(Shape::Socket), "send") => RtOp::NetSend,
            (Some(Shape::Socket), "receive") => RtOp::NetReceive,
            (Some(Shape::Bytes), "get") => RtOp::BytesGet,
            (Some(Shape::Bytes), "index_of") => RtOp::BytesIndexOf,
            (Some(Shape::Bytes), "text") => RtOp::BytesText,
            (Some(Shape::Bytes), "to_list") => RtOp::BytesToList,
            (Some(Shape::Bytes), "as_list") => RtOp::BytesAsList,
            (Some(Shape::Bytes), "crc32c") => RtOp::BytesCrc32c,
            (Some(Shape::Bytes), "xxhash64") => RtOp::BytesXxhash64,
            (Some(Shape::Channel(_)), "take") => RtOp::ChanTake,
            (None, "wrapping_add") if is_int => RtOp::WrappingAdd,
            (None, "wrapping_sub") if is_int => RtOp::WrappingSub,
            (None, "wrapping_mul") if is_int => RtOp::WrappingMul,
            (None, "checked_add") if is_int => RtOp::CheckedAdd,
            (None, "checked_sub") if is_int => RtOp::CheckedSub,
            (None, "checked_mul") if is_int => RtOp::CheckedMul,
            (None, "checked_div") if is_int => RtOp::CheckedDiv,
            (Some(Shape::Optional(_)), "unwrap") => {
                return Ok(ir::Expr {
                    kind: K::Unwrap(Box::new(values.remove(0))),
                    ty,
                    line,
                });
            }
            (Some(Shape::Optional(_)), "is_some" | "is_none") => {
                let present = ir::Expr {
                    kind: K::IsSome(Box::new(values.remove(0))),
                    ty: Type::Bool,
                    line,
                };
                if name == "is_some" {
                    return Ok(present);
                }
                return Ok(ir::Expr {
                    kind: K::Unary(UnOp::Not, Box::new(present)),
                    ty,
                    line,
                });
            }
            _ => {
                let unsupported = self.unsupported(format!("the method `{name}`"), span);
                return self
                    .more_method(name, values, ty, line)
                    .unwrap_or(Err(unsupported));
            }
        };
        // A method that changes its receiver gives back the changed value,
        // which the caller stores (`lower_stmt.rs`).
        let ty = if op.changes_receiver() {
            values[0].ty
        } else {
            ty
        };
        Ok(ir::Expr {
            kind: K::Rt(op, values),
            ty,
            line,
        })
    }
}
