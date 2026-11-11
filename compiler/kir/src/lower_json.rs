//! Lowering `Json` and `std.json` (spec 2.12). A `Json` literal is built
//! from the union's variants right here. Other values are encoded by the
//! runtime: they go in a one-item tuple, whose type says how to encode
//! them. `json.parse(text, max_depth: …)` takes its three limits as one
//! list operand, defaults filled in, so the op keeps the usual operands.

use crate::ir::{self, ExprKind as K, Shape, Type};
use crate::lower::{Lowered, Lowerer, Owner};
use crate::rt_op::RtOp;
use kavach_ast as ast;
use kavach_runtime_core::json::Limits;

/// A placeholder of type `t`: 0, or no object for a heap type.
fn zero(t: Type) -> K {
    match t {
        Type::F64 => K::Float(0.0),
        Type::Bool => K::Bool(false),
        Type::Char => K::Char('\0'),
        _ => K::Int(0),
    }
}

impl<'a> Lowerer<'a> {
    /// `json.decode<T>(text, limits…)` and `json.decode_value<T>(j)`: the
    /// runtime decodes by `T`'s shape, given the record defaults it needs
    /// as templates (`runtime/native/src/rt_json_decode.rs`).
    fn json_decode(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let params: &[&str] = if name == "decode" {
            &["text", "max_depth", "max_bytes", "max_fields"]
        } else {
            &["value"]
        };
        let mut given: Vec<Option<&'a ast::Expr>> = vec![None; params.len()];
        for (i, a) in args.iter().enumerate() {
            let p = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(slot) = p.and_then(|p| given.get_mut(p)) {
                *slot = Some(&a.value);
            }
        }
        let Some(first) = given[0] else {
            return Err(self.unsupported(format!("`json.{name}()`"), span));
        };
        let first = self.expr(first)?;
        let (op, limits) = if name == "decode" {
            let d = Limits::default();
            let mut limits = Vec::new();
            for (e, default) in given[1..]
                .iter()
                .zip([d.max_depth, d.max_bytes, d.max_fields])
            {
                limits.push(match e {
                    Some(e) => self.expr(e)?,
                    None => at(K::Int(default as i128), Type::I64),
                });
            }
            let list = self.list_type(Type::I64);
            (RtOp::JsonDecode, at(K::List(limits), list))
        } else {
            (RtOp::JsonDecodeValue, at(K::Int(0), Type::I64))
        };
        let templates = self.templates(ty, line, span)?;
        Ok(at(K::Rt(op, vec![first, limits, templates]), ty))
    }

    /// For each record type reachable from `ty` whose fields have
    /// defaults: a record holding them (0 in the other fields) and the
    /// mask of those fields, all in one tuple; or 0 when there are none.
    pub(crate) fn templates(&mut self, ty: Type, line: u32, span: ast::Span) -> Lowered<ir::Expr> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let (mut seen, mut todo, mut items) = (Vec::new(), vec![ty], Vec::new());
        while let Some(t) = todo.pop() {
            if seen.contains(&t) {
                continue;
            }
            seen.push(t);
            let Some(shape) = self.shapes.shape(t).cloned() else {
                continue;
            };
            match &shape {
                Shape::List(x) | Shape::Set(x) | Shape::Optional(x) => todo.push(*x),
                Shape::Map(k, v) => todo.extend([*k, *v]),
                Shape::Tuple(ts) => todo.extend(ts),
                Shape::Union { variants, .. } => {
                    todo.extend(variants.iter().flat_map(|v| v.fields.iter().map(|f| f.ty)))
                }
                Shape::Record { fields, .. } => {
                    todo.extend(fields.iter().map(|f| f.ty));
                    if let Some(template) = self.template(t, line, span)? {
                        items.extend(template);
                    }
                }
                _ => {}
            }
        }
        if items.is_empty() {
            return Ok(at(K::Int(0), Type::I64));
        }
        let types = items.iter().map(|e: &ir::Expr| e.ty).collect();
        let tuple = self.tuple_type(types);
        Ok(at(K::Tuple(items), tuple))
    }

    /// The template of record type `record` and its mask, if any field
    /// has a default.
    fn template(
        &mut self,
        record: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<Option<[ir::Expr; 2]>> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        let Some(def) = self.shapes.def_of(record) else {
            return Ok(None);
        };
        let decl_span = self.c.resolution.defs[def].span;
        let decl = self.c.file.items.iter().find_map(|item| match &item.kind {
            ast::ItemKind::Type(t) if Some(t.name.span) == decl_span => Some(t),
            _ => None,
        });
        let Some(ast::TypeBody::Record(r)) = decl.map(|d| &d.body) else {
            return Ok(None);
        };
        if r.fields.iter().all(|f| f.default.is_none()) {
            return Ok(None);
        }
        let types: Vec<Type> = match self.shapes.shape(record) {
            Some(Shape::Record { fields, .. }) => fields.iter().map(|f| f.ty).collect(),
            _ => return Err(self.unsupported("this record", span)),
        };
        let (mut values, mut mask) = (Vec::new(), 0i128);
        for (i, (f, t)) in r.fields.iter().zip(types).enumerate() {
            let value = match &f.default {
                Some(d) => {
                    mask |= 1 << i.min(63);
                    let v = self.expr(d)?;
                    self.coerce(v, t)?
                }
                // Never read: a field without a default is decoded or fails.
                None => at(zero(t), t),
            };
            values.push(value);
        }
        let fields = K::Record {
            variant: 0,
            fields: values,
        };
        Ok(Some([at(fields, record), at(K::Int(mask), Type::I64)]))
    }

    /// Whether the checker gave `e` the type `Json`.
    pub(crate) fn is_json_typed(&self, e: &ast::Expr) -> bool {
        let json = self.c.resolution.builtin_type("std.json", "Json");
        matches!(self.c.expr_types.get(&(e.span.start, e.span.end)),
            Some(kavach_checker::Ty::Adt(d, _)) if Some(*d) == json)
    }

    /// `value` in a one-item tuple, which tells the runtime its type.
    fn in_tuple(&mut self, value: ir::Expr) -> ir::Expr {
        let (ty, line) = (self.tuple_type(vec![value.ty]), value.line);
        ir::Expr {
            kind: K::Tuple(vec![value]),
            ty,
            line,
        }
    }

    /// `value` encoded as a `Json` of type `json` (`json.of`).
    fn encoded(&mut self, value: ir::Expr, json: Type) -> ir::Expr {
        let line = value.line;
        let operand = self.in_tuple(value);
        ir::Expr {
            kind: K::Rt(RtOp::JsonOf, vec![operand]),
            ty: json,
            line,
        }
    }

    /// Variant `name` of the `Json` union `json`, with one field or none.
    pub(crate) fn json_variant(
        &self,
        json: Type,
        name: &str,
        field: Option<ir::Expr>,
        line: u32,
    ) -> ir::Expr {
        let variant = match self.shapes.shape(json) {
            Some(Shape::Union { variants, .. }) => {
                variants.iter().position(|v| v.name == name).unwrap_or(0)
            }
            _ => 0,
        };
        ir::Expr {
            kind: K::Record {
                variant: variant as u32,
                fields: field.into_iter().collect(),
            },
            ty: json,
            line,
        }
    }

    /// The type of the field of `Json` variant `name`.
    pub(crate) fn json_field(&self, json: Type, name: &str) -> Type {
        match self.shapes.shape(json) {
            Some(Shape::Union { variants, .. }) => variants
                .iter()
                .find(|v| v.name == name)
                .and_then(|v| v.fields.first())
                .map_or(Type::Unit, |f| f.ty),
            _ => Type::Unit,
        }
    }

    /// A literal where a `Json` is expected, as a `Json` of type `json`.
    pub(crate) fn json_literal(&mut self, e: &'a ast::Expr, json: Type) -> Lowered<ir::Expr> {
        let line = self.line(e.span);
        let at = |kind, ty| ir::Expr { kind, ty, line };
        // A number literal keeps the digits it is written with (decision
        // 0071): `0.1` is `0.1`, not the nearest `Float`.
        let digits_ty = self.json_field(json, "Number");
        let number = |t: &str, negative: bool| {
            let digits: String = t.chars().filter(|c| *c != '_').collect();
            let digits = if negative {
                format!("-{digits}")
            } else {
                digits
            };
            ("Number", at(K::Str(digits), digits_ty))
        };
        let (name, field) = match &e.kind {
            ast::ExprKind::Int(n) => number(&n.to_string(), false),
            ast::ExprKind::Float(t) => number(t, false),
            ast::ExprKind::Unary(_, x) => match &x.kind {
                ast::ExprKind::Int(n) => number(&n.to_string(), true),
                ast::ExprKind::Float(t) => number(t, true),
                _ => return Ok(self.json_variant(json, "Null", None, line)),
            },
            ast::ExprKind::Bool(b) => ("Boolean", at(K::Bool(*b), Type::Bool)),
            ast::ExprKind::Str(parts) => {
                let kind = self.string(parts)?;
                ("Text", at(kind, self.json_field(json, "Text")))
            }
            ast::ExprKind::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.json_item(item, json)?);
                }
                ("Array", at(K::List(out), self.json_field(json, "Array")))
            }
            ast::ExprKind::Map(pairs) => {
                let mut out = Vec::with_capacity(pairs.len());
                for (k, v) in pairs {
                    out.push((self.expr(k)?, self.json_item(v, json)?));
                }
                ("Object", at(K::Map(out), self.json_field(json, "Object")))
            }
            _ => return Ok(self.json_variant(json, "Null", None, line)),
        };
        Ok(self.json_variant(json, name, Some(field), line))
    }

    /// An item or field inside a `Json` literal: another literal, a
    /// `Json`, or any value that can be encoded.
    fn json_item(&mut self, e: &'a ast::Expr, json: Type) -> Lowered<ir::Expr> {
        if kavach_checker::json_literal_kind(e) {
            return self.json_literal(e, json);
        }
        let value = self.expr(e)?;
        if self.is_json_typed(e) {
            return Ok(value);
        }
        Ok(self.encoded(value, json))
    }

    /// `j["key"]` and `j[i]`: what the `get` and `at` methods of `Json`
    /// do (`stdlib/json/json.kav`), read by the runtime (decision 0032).
    pub(crate) fn json_index(
        &mut self,
        base: ir::Expr,
        index: &'a ast::Expr,
        _span: ast::Span,
    ) -> Lowered<K> {
        let index = self.expr(index)?;
        let op = if index.ty == Type::I64 {
            RtOp::JsonAt
        } else {
            RtOp::JsonGet
        };
        Ok(K::Rt(op, vec![base, index]))
    }

    /// `j.get(key)`, `j.at(i)`, `j.list()` and `j.object()` of a `Json`
    /// whose type is `owner`: read by the runtime, as `json_index`.
    pub(crate) fn json_method(
        &mut self,
        owner: Option<&Owner>,
        name: &str,
        receiver: &ir::Expr,
        args: &'a [ast::Arg],
    ) -> Lowered<Option<(RtOp, Vec<ir::Expr>)>> {
        let json = self.c.resolution.builtin_type("std.json", "Json");
        if json.is_none() || owner != json.map(Owner::Adt).as_ref() {
            return Ok(None);
        }
        let op = match (name, args.len()) {
            ("get", 1) => RtOp::JsonGet,
            ("at", 1) => RtOp::JsonAt,
            ("list", 0) => RtOp::JsonList,
            ("object", 0) => RtOp::JsonObject,
            ("int", 0) => RtOp::JsonInt,
            ("float", 0) => RtOp::JsonFloat,
            ("text", 0) => RtOp::JsonText,
            ("bool", 0) => RtOp::JsonBool,
            ("is_null", 0) => RtOp::JsonIsNull,
            ("len", 0) => RtOp::JsonLen,
            _ => return Ok(None),
        };
        let mut operands = vec![receiver.clone()];
        if let Some(a) = args.first() {
            operands.push(self.expr(&a.value)?);
        }
        Ok(Some((op, operands)))
    }

    pub(crate) fn json_call(
        &mut self,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let at = |kind, ty| ir::Expr { kind, ty, line };
        if matches!(name, "decode" | "decode_value") {
            return self.json_decode(name, args, ty, line, span);
        }
        if name == "of" {
            let value = match args.first() {
                Some(a) => self.expr(&a.value)?,
                None => return Err(self.unsupported("`json.of()`", span)),
            };
            return Ok(self.encoded(value, ty));
        }
        let (op, params): (RtOp, &[&str]) = match name {
            "parse" => (
                RtOp::JsonParse,
                &["text", "max_depth", "max_bytes", "max_fields", "comments"],
            ),
            "stringify" => (RtOp::JsonStringify, &["value", "indent"]),
            _ => return Err(self.unsupported(format!("`json.{name}`"), span)),
        };
        // The arguments in parameter order: positional, then named.
        let mut given: Vec<Option<&'a ast::Expr>> = vec![None; params.len()];
        for (i, a) in args.iter().enumerate() {
            let at_param = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => Some(i),
            };
            if let Some(slot) = at_param.and_then(|p| given.get_mut(p)) {
                *slot = Some(&a.value);
            }
        }
        let mut value = |e: Option<&'a ast::Expr>, default: usize| match e {
            Some(e) => self.expr(e),
            None => Ok(at(K::Int(default as i128), Type::I64)),
        };
        let first = value(given[0], 0)?;
        let last = if op == RtOp::JsonParse {
            let d = Limits::default();
            let mut limits = vec![
                value(given[1], d.max_depth)?,
                value(given[2], d.max_bytes)?,
                value(given[3], d.max_fields)?,
            ];
            // `comments: c` goes in the list as 1 or 0.
            if let Some(c) = given.get(4).copied().flatten() {
                let one = |n: i128| ir::Block {
                    stmts: Vec::new(),
                    value: Some(Box::new(at(K::Int(n), Type::I64))),
                };
                let c = self.expr(c)?;
                limits.push(at(K::If(Box::new(c), one(1), one(0)), Type::I64));
            }
            let list = self.list_type(Type::I64);
            at(K::List(limits), list)
        } else {
            value(given[1], 0)?
        };
        // `stringify` encodes any value, so it needs the value's type.
        let first = if op == RtOp::JsonStringify {
            self.in_tuple(first)
        } else {
            first
        };
        let values = vec![first, last];
        Ok(at(K::Rt(op, values), ty))
    }
}
