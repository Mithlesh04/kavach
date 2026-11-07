//! Lowering expressions and `if`. Calls, data, `match`, errors and
//! lambdas each have their own `lower_*.rs` file.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, BinaryOp, ExprKind, StrPart, UnaryOp};
use kavach_resolver::DefKind;

impl<'a> Lowerer<'a> {
    /// The KIR type of `e`, from the checker.
    pub(crate) fn ty_of(&mut self, e: &ast::Expr) -> Lowered<Type> {
        match self.c.expr_types.get(&(e.span.start, e.span.end)).cloned() {
            Some(ty) => self.kir_type(&ty, e.span),
            None => Ok(Type::Unit),
        }
    }

    pub(crate) fn expr(&mut self, e: &'a ast::Expr) -> Lowered<ir::Expr> {
        let ty = self.ty_of(e)?;
        self.expr_as(e, ty)
    }

    /// Lowers `e`, whose type is `ty`. (A dot lambda and its body share a
    /// span, so the checker's type for that span is the lambda's.)
    pub(crate) fn expr_as(&mut self, e: &'a ast::Expr, ty: Type) -> Lowered<ir::Expr> {
        let line = self.line(e.span);
        if kavach_checker::json_literal_kind(e) && self.is_json_typed(e) {
            return self.json_literal(e, ty);
        }
        // `Decimal` and `BigInt` literals are made from their text; a
        // `BigInt` constant the checker worked out from its digits.
        let is_decimal = matches!(self.shapes.shape(ty), Some(Shape::Decimal | Shape::BigInt));
        if let (Some(f), Type::F64) = (self.c.float_constants.get(&(e.span.start, e.span.end)), ty)
        {
            return Ok(ir::Expr {
                kind: K::Float(*f),
                ty,
                line,
            });
        }
        if let Some(digits) = self.c.big_constants.get(&(e.span.start, e.span.end)) {
            return Ok(ir::Expr {
                kind: K::Decimal(digits.clone()),
                ty,
                line,
            });
        }
        let kind = match &e.kind {
            ExprKind::BigInt(digits) if ty.is_float() => K::Float(digits.parse().unwrap_or(0.0)),
            ExprKind::BigInt(digits) => K::Decimal(digits.clone()),
            ExprKind::Int(n) if ty == Type::F64 => K::Float(*n as f64),
            ExprKind::Int(n) if ty == Type::F32 => K::Float(f64::from(*n as f32)),
            // Read straight as single precision: rounded once (spec 2.1).
            ExprKind::Float(text) if ty == Type::F32 => K::Float(f64::from(
                text.parse::<f32>()
                    .map_err(|_| self.unsupported("this number", e.span))?,
            )),
            ExprKind::Int(n) if is_decimal => K::Decimal(n.to_string()),
            ExprKind::Float(text) if is_decimal => K::Decimal(text.clone()),
            ExprKind::Int(n) => K::Int(*n as i128),
            ExprKind::Float(text) => K::Float(
                text.parse()
                    .map_err(|_| self.unsupported("this number", e.span))?,
            ),
            ExprKind::Bool(b) => K::Bool(*b),
            ExprKind::Char(c) => K::Char(*c),
            ExprKind::Str(parts) => self.string(parts)?,
            ExprKind::None => K::None,
            ExprKind::SelfValue => K::Local(
                self.self_local
                    .ok_or_else(|| self.unsupported("`self` here", e.span))?,
            ),
            ExprKind::Name(id) => return self.name(id, ty, line),
            ExprKind::It => K::Local(
                *self
                    .its
                    .last()
                    .ok_or_else(|| self.unsupported("`.` outside a lambda", e.span))?,
            ),
            ExprKind::Try { expr, context } => {
                return self.try_expr(expr, context.as_deref(), line);
            }
            ExprKind::Catch {
                expr,
                binding,
                handler,
            } => return self.catch_expr(expr, binding.as_ref(), handler, line),
            ExprKind::Fail(x) => return self.fail_expr(x, line),
            ExprKind::Spawn(kind, x, within) => {
                return self.spawn_within(*kind, x, within.as_deref(), ty, line);
            }
            ExprKind::Lambda { .. } | ExprKind::DotLambda(_) => return self.closure(e, ty, line),
            // `*p` (spec 6.4).
            ExprKind::Unary(UnaryOp::Deref, x) => return self.ptr_load(x, None, line),
            // `-128` as an `Int8` is one literal: `128` alone doesn't fit.
            ExprKind::Unary(UnaryOp::Neg, x)
                if ty.is_int() && matches!(x.kind, ExprKind::Int(_)) =>
            {
                let ExprKind::Int(n) = x.kind else {
                    unreachable!("matched above")
                };
                K::Int(-(n as i128))
            }
            ExprKind::Unary(op, x) => {
                let op = match op {
                    UnaryOp::Neg => UnOp::Neg,
                    UnaryOp::Not => UnOp::Not,
                    UnaryOp::BitNot => UnOp::BitNot,
                    UnaryOp::Deref => unreachable!("lowered above"),
                };
                K::Unary(op, Box::new(self.expr(x)?))
            }
            ExprKind::Binary(BinaryOp::Default, l, r) => return self.default_value(l, r, ty, line),
            // `p + n`, `p - n` (spec 6.4).
            ExprKind::Binary(op @ (BinaryOp::Add | BinaryOp::Sub), l, r) if self.is_ptr(l) => {
                return self.ptr_moved(l, r, *op == BinaryOp::Sub, line);
            }
            ExprKind::Binary(op, l, r) => {
                let (l, r) = (self.expr(l)?, self.expr(r)?);
                // `T` compared with `T?`: compare as optionals.
                let r = self.coerce(r, l.ty)?;
                let l = self.coerce(l, r.ty)?;
                if *op == BinaryOp::Add && matches!(self.shapes.shape(l.ty), Some(Shape::Bytes)) {
                    return Ok(ir::Expr {
                        kind: K::Rt(RtOp::BytesConcat, vec![l, r]),
                        ty,
                        line,
                    });
                }
                if *op == BinaryOp::Pow {
                    return Ok(self.power(l, r, line));
                }
                let (l, r) = (Box::new(l), Box::new(r));
                match op {
                    BinaryOp::And => K::And(l, r),
                    BinaryOp::Or => K::Or(l, r),
                    _ => K::Binary(
                        binary_op(*op).ok_or_else(|| {
                            self.unsupported(format!("`{}`", op.as_str()), e.span)
                        })?,
                        l,
                        r,
                    ),
                }
            }
            ExprKind::Compare(first, links) => return self.chain(first, links, e.span),
            ExprKind::AnonRecord(fs) => {
                // The shape's fields are sorted by name (spec 2.14);
                // build in that order.
                let Some(Shape::Record { fields, .. }) = self.shapes.shape(ty).cloned() else {
                    return Err(self.unsupported("this record here", e.span));
                };
                let mut exprs = Vec::new();
                for f in &fields {
                    let (_, x) = fs
                        .iter()
                        .find(|(n, _)| n.name == f.name)
                        .ok_or_else(|| self.unsupported("this record here", e.span))?;
                    let v = self.expr(x)?;
                    exprs.push(self.coerce(v, f.ty)?);
                }
                K::Record {
                    variant: 0,
                    fields: exprs,
                }
            }
            ExprKind::Call { callee, args, .. } => return self.call(e, callee, args, ty),
            ExprKind::Field {
                base,
                name,
                optional,
            } => return self.field(base, name, *optional, ty, line),
            ExprKind::TupleField { base, index } => {
                let base = self.expr(base)?;
                let index = ir::Expr {
                    kind: K::Int(*index as i128),
                    ty: Type::I64,
                    line,
                };
                K::Index(Box::new(base), Box::new(index))
            }
            ExprKind::Index { base, index } if self.is_ptr(base) => {
                return self.ptr_load(base, Some(index), line);
            }
            ExprKind::Index { base, index } => {
                let base = self.expr(base)?;
                match self.shapes.shape(base.ty).cloned() {
                    Some(Shape::List(_)) => K::Index(Box::new(base), Box::new(self.expr(index)?)),
                    Some(Shape::Bytes) => K::Rt(RtOp::BytesAt, vec![base, self.expr(index)?]),
                    // `map[key]` is an optional (spec 2.2).
                    Some(Shape::Map(k, _)) => {
                        let key = self.expr(index)?;
                        K::Rt(RtOp::MapGet, vec![base, self.coerce(key, k)?])
                    }
                    // `j["key"]` and `j[i]` on a `Json` (spec 2.12).
                    Some(Shape::Union { .. }) => self.json_index(base, index, e.span)?,
                    _ => return Err(self.unsupported("indexing this", e.span)),
                }
            }
            ExprKind::With { base, fields } => {
                let mut value = self.expr(base)?;
                for (name, v) in fields {
                    let (index, field_ty) = self.field_index(value.ty, &name.name, name.span)?;
                    let v = self.expr(v)?;
                    let v = self.coerce(v, field_ty)?;
                    value = ir::Expr {
                        kind: K::SetField(Box::new(value), index, Box::new(v)),
                        ty,
                        line,
                    };
                }
                return self.with_invariants(value);
            }
            ExprKind::List(items) => {
                let Some(Shape::List(item_ty)) = self.shapes.shape(ty).cloned() else {
                    return Err(self.unsupported("this list", e.span));
                };
                let mut values = Vec::new();
                for x in items {
                    let v = self.expr(x)?;
                    values.push(self.coerce(v, item_ty)?);
                }
                K::List(values)
            }
            ExprKind::Map(entries) => {
                let Some(Shape::Map(k, v)) = self.shapes.shape(ty).cloned() else {
                    return Err(self.unsupported("this map", e.span));
                };
                let mut pairs = Vec::new();
                for (key, value) in entries {
                    let (key, value) = (self.expr(key)?, self.expr(value)?);
                    pairs.push((self.coerce(key, k)?, self.coerce(value, v)?));
                }
                K::Map(pairs)
            }
            ExprKind::Tuple(items) => {
                K::Tuple(items.iter().map(|x| self.expr(x)).collect::<Lowered<_>>()?)
            }
            ExprKind::If { .. }
            | ExprKind::IfLet { .. }
            | ExprKind::Match { .. }
            | ExprKind::Block(_)
            | ExprKind::Arena(_)
            | ExprKind::Unsafe(_) => {
                return self.branching(e, true);
            }
            _ => {
                return Err(
                    self.unsupported(format!("`{}`", first_word(self.text(e.span))), e.span)
                );
            }
        };
        Ok(ir::Expr { kind, ty, line })
    }

    /// A string literal, or interpolation: the printed form of each part.
    pub(crate) fn string(&mut self, parts: &'a [StrPart]) -> Lowered<K> {
        match parts {
            [] => return Ok(K::Str(String::new())),
            [StrPart::Text(t)] => return Ok(K::Str(t.clone())),
            _ => {}
        }
        let str_ty = self.str_type();
        let mut values = Vec::new();
        for p in parts {
            values.push(match p {
                StrPart::Text(t) => ir::Expr {
                    kind: K::Str(t.clone()),
                    ty: str_ty,
                    line: 0,
                },
                StrPart::Expr(x) => self.expr(x)?,
            });
        }
        Ok(K::Format(values))
    }

    fn name(&mut self, id: &'a ast::Ident, ty: Type, line: u32) -> Lowered<ir::Expr> {
        let def = self.c.resolution.uses.get(&id.span.start).copied();
        match def.map(|d| self.c.resolution.defs[d].kind) {
            Some(DefKind::Variant { .. }) => {
                self.variant_value(ty, &id.name, Vec::new(), line, id.span)
            }
            Some(DefKind::Fn) => match self.c.resolution.uses.get(&id.span.start) {
                Some(d) if self.fn_ids.contains_key(d) => {
                    let fid = self.fn_ids[d];
                    self.function_value(fid, ty, id.span)
                }
                Some(&d) if self.generic_fns.contains_key(&d) => {
                    self.generic_fn_value(d, ty, id.span)
                }
                _ => Err(self.unsupported("generic functions as values", id.span)),
            },
            Some(DefKind::Builtin) if id.name == "count" => self
                .rule_count(line)
                .ok_or_else(|| self.unsupported("`count` outside a `limit` rule", id.span)),
            Some(DefKind::Builtin) if id.name == "tokens" => self
                .rule_tokens(line)
                .ok_or_else(|| self.unsupported("`tokens` outside a `limit` rule", id.span)),
            Some(DefKind::Field) => {
                // A field name inside an `invariant` reads the record being checked.
                let (local, record_ty) = self
                    .invariant_of
                    .ok_or_else(|| self.unsupported("this field", id.span))?;
                let (index, _) = self.field_index(record_ty, &id.name, id.span)?;
                let record = self.local_expr(local, line);
                Ok(ir::Expr {
                    kind: K::GetField(Box::new(record), index),
                    ty,
                    line,
                })
            }
            None if self.c.resolution.ambiguous.contains_key(&id.span.start) => {
                self.variant_value(ty, &id.name, Vec::new(), line, id.span)
            }
            _ => Ok(ir::Expr {
                kind: K::Local(self.local(id)?),
                ty,
                line,
            }),
        }
    }

    /// `if`, `if let`, `match` and blocks. Without `want_value` their
    /// branches are statements.
    pub(crate) fn branching(&mut self, e: &'a ast::Expr, want_value: bool) -> Lowered<ir::Expr> {
        let line = self.line(e.span);
        let ty = if want_value {
            self.ty_of(e)?
        } else {
            Type::Unit
        };
        let kind = match &e.kind {
            // `unsafe` (spec 6.4) only lets the checker allow more inside;
            // the code is a block like any other.
            ExprKind::Block(b) | ExprKind::Arena(b) | ExprKind::Unsafe(b) => {
                K::Block(self.branch_block(b, want_value, ty)?)
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                let cond = self.expr(cond)?;
                let then_block = self.branch_block(then, want_value, ty)?;
                let else_block = self.else_block(otherwise.as_deref(), want_value, ty)?;
                K::If(Box::new(cond), then_block, else_block)
            }
            ExprKind::IfLet {
                name,
                value,
                then,
                otherwise,
            } => {
                let value = self.expr(value)?;
                let Some(Shape::Optional(inner)) = self.shapes.shape(value.ty).cloned() else {
                    return Err(self.unsupported("this `if let`", e.span));
                };
                let tmp = self.new_local(value.ty);
                let bound = self.new_local(inner);
                self.bind(name, bound);
                let mut then_block = self.branch_block(then, want_value, ty)?;
                let unwrapped = ir::Expr {
                    kind: K::Unwrap(Box::new(self.local_expr(tmp, line))),
                    ty: inner,
                    line,
                };
                then_block.stmts.insert(0, Stmt::Let(bound, unwrapped));
                let else_block = self.else_block(otherwise.as_deref(), want_value, ty)?;
                let present = ir::Expr {
                    kind: K::IsSome(Box::new(self.local_expr(tmp, line))),
                    ty: Type::Bool,
                    line,
                };
                let choice = ir::Expr {
                    kind: K::If(Box::new(present), then_block, else_block),
                    ty,
                    line,
                };
                K::Block(Block {
                    stmts: vec![Stmt::Let(tmp, value)],
                    value: Some(Box::new(choice)),
                })
            }
            ExprKind::Match { scrutinee, arms } => {
                return self.match_expr(scrutinee, arms, ty, want_value, line);
            }
            _ => return Err(self.unsupported("this expression", e.span)),
        };
        Ok(ir::Expr { kind, ty, line })
    }

    fn else_block(
        &mut self,
        otherwise: Option<&'a ast::Expr>,
        want_value: bool,
        ty: Type,
    ) -> Lowered<Block> {
        let Some(o) = otherwise else {
            return Ok(Block::default());
        };
        let other = self.branching(o, want_value)?;
        let other = self.coerce(other, ty)?;
        Ok(wrap(other, want_value))
    }

    pub(crate) fn branch_block(
        &mut self,
        b: &'a ast::Block,
        want_value: bool,
        ty: Type,
    ) -> Lowered<Block> {
        let mut block = Block::default();
        self.block_into(b, want_value, &mut block)?;
        if let Some(v) = block.value.take() {
            block.value = Some(Box::new(self.coerce(*v, ty)?));
        }
        Ok(block)
    }
}

/// A block holding `e` as its value, or as its only statement.
pub(crate) fn wrap(e: ir::Expr, as_value: bool) -> Block {
    if as_value {
        Block {
            stmts: Vec::new(),
            value: Some(Box::new(e)),
        }
    } else {
        Block {
            stmts: vec![Stmt::Expr(e)],
            value: None,
        }
    }
}

pub(crate) fn binary_op(op: BinaryOp) -> Option<BinOp> {
    Some(match op {
        BinaryOp::Add => BinOp::Add,
        BinaryOp::Sub => BinOp::Sub,
        BinaryOp::Mul => BinOp::Mul,
        BinaryOp::Div => BinOp::Div,
        BinaryOp::Rem => BinOp::Rem,
        BinaryOp::Eq => BinOp::Eq,
        BinaryOp::NotEq => BinOp::Ne,
        BinaryOp::Lt => BinOp::Lt,
        BinaryOp::Le => BinOp::Le,
        BinaryOp::Gt => BinOp::Gt,
        BinaryOp::Ge => BinOp::Ge,
        BinaryOp::BitAnd => BinOp::BitAnd,
        BinaryOp::BitOr => BinOp::BitOr,
        BinaryOp::BitXor => BinOp::BitXor,
        BinaryOp::Shl => BinOp::Shl,
        BinaryOp::Shr => BinOp::Shr,
        _ => return None,
    })
}

pub(crate) fn first_word(text: &str) -> &str {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .find(|w| !w.is_empty())
        .unwrap_or(text)
}
