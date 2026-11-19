// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Records passed to C and given back by value (spec 10.7): each copied
//! into C memory, as a `Ptr` write does, and passed as this machine's C
//! passes a `struct` (`c_abi.rs`): its eightbytes in registers, the
//! address of a copy, or a copy on the stack. A record result comes back
//! in registers or through a hidden pointer, and is read from memory.

use crate::c_abi::{self, Layout, Passed, Returned, Rules};
use crate::ir::{self, Block, ExprKind as K, FastOp, Stmt, Type};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;

fn at(kind: K, ty: Type, line: u32) -> ir::Expr {
    ir::Expr { kind, ty, line }
}

impl<'a> Lowerer<'a> {
    /// A call of the C function at `target` with `values`, giving `ty`,
    /// when a record goes by value either way; the values back when none
    /// does.
    pub(crate) fn c_record_call(
        &mut self,
        target: ir::Expr,
        values: Vec<ir::Expr>,
        ty: Type,
        line: u32,
        span: kavach_ast::Span,
    ) -> Lowered<Result<ir::Expr, Vec<ir::Expr>>> {
        let layouts: Vec<_> = values.iter().map(|v| self.c_layout(v.ty)).collect();
        let result = self.c_layout(ty);
        if result.is_none() && layouts.iter().all(Option::is_none) {
            return Ok(Err(values));
        }
        let Some(rules) = Rules::here() else {
            return Err(self.unsupported("a record by value to C on this machine", span));
        };
        let returned = (result.as_ref()).map(|(size, fields)| {
            c_abi::returned(
                rules,
                &Layout {
                    size: *size,
                    fields,
                },
            )
        });
        let hidden = matches!(returned, Some(Returned::Hidden));
        let kinds: Vec<Result<Layout, Type>> = (layouts.iter().zip(&values))
            .map(|(l, v)| match l {
                Some((size, fields)) => Ok(Layout {
                    size: *size,
                    fields,
                }),
                None => Err(v.ty),
            })
            .collect();
        let refs: Vec<Result<&Layout, Type>> =
            kinds.iter().map(|k| k.as_ref().map_err(|t| *t)).collect();
        let passed = c_abi::passed(rules, &refs, hidden);
        let (mut stmts, mut memory) = (Vec::new(), Vec::new());
        let (mut words, mut operands) = (Vec::new(), Vec::new());
        let out = match (&returned, &result) {
            (Some(r), Some((size, _))) => {
                let mem = self.c_memory(*size, line, &mut stmts, &mut memory);
                words.push(match r {
                    Returned::Hidden => format!("h{size}"),
                    Returned::Parts(_) => "m".into(),
                });
                operands.push(self.local_expr(mem, line));
                Some(mem)
            }
            _ => None,
        };
        for ((v, how), layout) in values.into_iter().zip(&passed).zip(&layouts) {
            let (Some(how), Some((size, fields))) = (how, layout) else {
                words.push("n".into());
                operands.push(v);
                continue;
            };
            let mem = self.c_memory(*size, line, &mut stmts, &mut memory);
            let address = self.local_expr(mem, line);
            let write = self.record_access(v.ty, fields, address, Some(v), line)?;
            stmts.push(Stmt::Expr(write));
            words.extend(c_abi::operand_words(how, *size));
            match how {
                Passed::Parts(parts) => {
                    for (k, part) in parts.iter().enumerate() {
                        let t = match part {
                            c_abi::Part::Int => Type::I64,
                            c_abi::Part::Float => Type::F64,
                        };
                        let offset = Box::new(at(K::Int(8 * k as i128), Type::I64, line));
                        let base = Box::new(self.local_expr(mem, line));
                        let addr = at(K::Fast(FastOp::Add, base, offset), Type::I64, line);
                        operands.push(at(K::Rt(RtOp::PtrRead, vec![addr]), t, line));
                    }
                }
                Passed::Copy | Passed::Stack => operands.push(self.local_expr(mem, line)),
            }
        }
        let result_word = match &returned {
            Some(r) => c_abi::result_word(r),
            None if ty == Type::Unit => "v".into(),
            None => "n".into(),
        };
        let text = self.str_type();
        let descriptor = at(K::Str(c_abi::descriptor(&result_word, &words)), text, line);
        let call_ty = if out.is_some() { Type::Unit } else { ty };
        let call = self.c_copied_call(operands, call_ty, line, |args| {
            K::Rt(
                RtOp::CallC,
                [descriptor, target].into_iter().chain(args).collect(),
            )
        });
        let value = match (out, &result) {
            (Some(mem), Some((_, fields))) => {
                stmts.push(Stmt::Expr(call));
                let address = self.local_expr(mem, line);
                let read = self.record_access(ty, fields, address, None, line)?;
                let got = self.new_local(ty);
                stmts.push(Stmt::Let(got, read));
                Some(got)
            }
            _ if ty == Type::Unit => {
                stmts.push(Stmt::Expr(call));
                None
            }
            _ => {
                let got = self.new_local(ty);
                stmts.push(Stmt::Let(got, call));
                Some(got)
            }
        };
        for mem in memory {
            let free = K::Rt(RtOp::PtrFree, vec![self.local_expr(mem, line)]);
            stmts.push(Stmt::Expr(at(free, Type::Unit, line)));
        }
        let value = value.map(|v| Box::new(self.local_expr(v, line)));
        Ok(Ok(at(K::Block(Block { stmts, value }), ty, line)))
    }

    /// C memory for a record of `size` bytes, at least 16 so a register
    /// part written whole fits, kept in a new local; freed after the call.
    fn c_memory(
        &mut self,
        size: i128,
        line: u32,
        stmts: &mut Vec<Stmt>,
        memory: &mut Vec<ir::Local>,
    ) -> ir::Local {
        let mem = self.new_local(Type::I64);
        let count = at(K::Int(1), Type::I64, line);
        let bytes = at(K::Int(size.max(16)), Type::I64, line);
        let alloc = at(K::Rt(RtOp::PtrAlloc, vec![count, bytes]), Type::I64, line);
        stmts.push(Stmt::Let(mem, alloc));
        memory.push(mem);
        mem
    }
}
