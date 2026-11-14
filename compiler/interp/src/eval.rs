//! Evaluating expressions.

use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Closure, Eval, Flow, Value};
use kavach_ast::*;
use kavach_checker::Prim;
use kavach_resolver::DefKind;
use kavach_runtime_core::Decimal;
use std::rc::Rc;

impl<'a> Interp<'a> {
    pub(crate) fn eval(&mut self, e: &'a Expr) -> Eval<'a> {
        if kavach_checker::json_literal_kind(e) && self.is_json_typed(e) {
            return self.json_literal(e);
        }
        if let Some(n) = self.big_constant(e) {
            return Ok(n);
        }
        match &e.kind {
            ExprKind::BigInt(digits) => Ok(match self.literal_type(e.span) {
                Prim::Decimal => match kavach_runtime_core::AnyDecimal::parse(digits) {
                    Some(d) => crate::bigint::decimal_value(d),
                    None => Value::Decimal(Decimal::from_int(0)),
                },
                Prim::Float32 => Value::Float32(digits.parse().unwrap_or(0.0)),
                Prim::Float => Value::Float(digits.parse().unwrap_or(0.0)),
                _ => Value::BigInt(Rc::new(digits.parse().unwrap_or_default())),
            }),
            ExprKind::Int(n) => Ok(match self.literal_type(e.span) {
                Prim::Float32 => Value::Float32(*n as f32),
                Prim::Float => Value::Float(*n as f64),
                Prim::BigInt => Value::BigInt(Rc::new((*n).into())),
                Prim::Decimal => Value::Decimal(Decimal::from_int(*n as i128)),
                p => Value::Int(*n as i128, p),
            }),
            ExprKind::Float(text) => Ok(match self.literal_type(e.span) {
                Prim::Decimal => {
                    let d = kavach_runtime_core::AnyDecimal::parse(text);
                    crate::bigint::decimal_value(
                        d.unwrap_or(kavach_runtime_core::AnyDecimal::Small(Decimal::from_int(0))),
                    )
                }
                Prim::Float32 => Value::Float32(text.parse().unwrap_or(0.0)),
                _ => Value::Float(text.parse().unwrap_or(0.0)),
            }),
            ExprKind::Char(c) => Ok(Value::Char(*c)),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::None => Ok(Value::None),
            // `sql"…"` (spec 10.15): its tag's record, holding the text.
            ExprKind::Tagged { args, .. } => {
                let def = match self.c.expr_types.get(&(e.span.start, e.span.end)) {
                    Some(kavach_checker::Ty::Adt(def, _)) => *def,
                    _ => return Err(self.panic("this tagged literal has no type", e.span)),
                };
                let mut values = Vec::new();
                for a in args {
                    values.push((None, self.eval(&a.value)?));
                }
                self.construct(def, None, values, e.span)
            }
            ExprKind::Str(parts) => {
                let mut s = String::new();
                for part in parts {
                    match part {
                        StrPart::Text(t) => s.push_str(t),
                        StrPart::Expr(x) => s.push_str(&self.eval(x)?.display()),
                    }
                }
                Ok(Value::Str(Rc::from(s)))
            }
            ExprKind::Name(id) => self.name(id, e.span),
            ExprKind::SelfValue => Ok(self
                .frames
                .last()
                .and_then(|f| f.self_value.clone())
                .unwrap_or(Value::Unit)),
            ExprKind::It => Ok(self
                .frames
                .last()
                .and_then(|f| f.its.last().cloned())
                .unwrap_or(Value::Unit)),
            ExprKind::List(items) => Ok(Value::List(Rc::new(self.eval_all(items)?))),
            ExprKind::Tuple(items) => Ok(Value::Tuple(self.eval_all(items)?)),
            ExprKind::Map(entries) => {
                let mut map = MapValue::default();
                for (k, v) in entries {
                    let (k, v) = (self.eval(k)?, self.eval(v)?);
                    map.insert(k, v);
                }
                Ok(Value::Map(Rc::new(map)))
            }
            ExprKind::AnonRecord(fs) => {
                // Fields are evaluated as written, kept sorted by name so
                // equal shapes compare equal (spec 2.14).
                let mut fields = Vec::new();
                for (name, x) in fs {
                    let value = self.eval(x)?;
                    fields.push(crate::value::Field {
                        name: Rc::from(name.name.as_str()),
                        value,
                        secret: false,
                    });
                }
                fields.sort_by(|a, b| a.name.cmp(&b.name));
                Ok(Value::Record {
                    def: usize::MAX,
                    name: Rc::from(""),
                    fields,
                    life: None,
                })
            }
            ExprKind::Unary(op, x) => {
                let v = self.eval(x)?;
                self.unary(*op, v, e.span)
            }
            ExprKind::Binary(op, l, r) => self.binary(*op, l, r, e.span),
            ExprKind::Compare(first, links) => self.chain(first, links, e.span),
            ExprKind::Call { callee, args, .. } => self.call(e, callee, args),
            ExprKind::Field {
                base,
                name,
                optional,
            } => {
                if let Some(v) = self.static_variant(base, name) {
                    return Ok(v);
                }
                if let Some(d) = self.duration(base, name, e.span) {
                    return d;
                }
                // `math.pi`: a constant of a standard module (spec 14.5).
                if let ExprKind::Name(id) = &base.kind {
                    let def = self.c.resolution.uses.get(&id.span.start);
                    if let Some(module) = def.and_then(|d| self.std_modules.get(d)) {
                        let v = self.std_constant(module, &name.name);
                        return v.ok_or_else(|| self.panic("no such constant", e.span));
                    }
                }
                let v = self.eval(base)?;
                if *optional && v == Value::None {
                    return Ok(Value::None);
                }
                self.get_field(&v, &name.name, e.span)
            }
            ExprKind::TupleField { base, index } => match self.eval(base)? {
                Value::Tuple(items) => {
                    Ok(items.get(*index as usize).cloned().unwrap_or(Value::Unit))
                }
                _ => Err(self.panic("not a tuple", e.span)),
            },
            ExprKind::Index { base, index } => {
                let (b, i) = (self.eval(base)?, self.eval(index)?);
                if let Value::Variant { union, .. } = &b
                    && crate::json::is_json(*union)
                {
                    return self.json_index(b, i, e.span);
                }
                self.index(b, i, e.span)
            }
            ExprKind::With { base, fields } => {
                let mut v = self.eval(base)?;
                if let Value::Record {
                    fields: fs, life, ..
                } = &mut v
                {
                    for (name, value) in fields {
                        let value = self.eval(value)?;
                        if let Some(f) = fs.iter_mut().find(|f| *f.name == *name.name) {
                            f.value = value;
                        }
                    }
                    // A new value, with an identity of its own (spec 6.1).
                    crate::drops::renew(life, fs, true);
                }
                self.check_invariants(&v, e.span)?;
                Ok(v)
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                if self.eval(cond)? == Value::Bool(true) {
                    self.block(then)
                } else {
                    otherwise.as_ref().map_or(Ok(Value::Unit), |o| self.eval(o))
                }
            }
            ExprKind::IfLet {
                name,
                value,
                then,
                otherwise,
            } => match self.eval(value)? {
                Value::None => otherwise.as_ref().map_or(Ok(Value::Unit), |o| self.eval(o)),
                v => {
                    let def = self.decl.get(&name.span.start).copied();
                    if let Some(def) = def {
                        self.frame().vars.insert(def, v);
                    }
                    let result = self.block(then);
                    // The bound value ends with its block (spec 6.1).
                    if let Some(def) = def {
                        self.frame().vars.remove(&def);
                    }
                    result
                }
            },
            ExprKind::Match { scrutinee, arms } => self.match_expr(scrutinee, arms, e.span),
            ExprKind::Block(b) | ExprKind::Arena(b) | ExprKind::Unsafe(b) => self.block(b),
            ExprKind::Scoped { call, body } => self.scoped(call, body),
            ExprKind::Lambda { params, body } => Ok(self.closure(
                params
                    .iter()
                    .map(|p| self.decl.get(&p.name.span.start).copied())
                    .collect(),
                body,
                false,
            )),
            ExprKind::DotLambda(body) => Ok(self.closure(Vec::new(), body, true)),
            ExprKind::Try { expr, context } => match self.eval(expr) {
                Err(Flow::Fail(err)) => {
                    let err = match context {
                        Some(c) => {
                            let text = self.eval(c)?.display();
                            Value::Context(Rc::from(text), Box::new(err))
                        }
                        None => err,
                    };
                    self.leaving = true;
                    Err(Flow::Fail(err))
                }
                other => other,
            },
            ExprKind::Catch {
                expr,
                binding,
                handler,
            } => {
                self.leaving = false;
                let caught = self.eval(expr);
                match caught {
                    // One raised by `fail` or `try` in an argument leaves the
                    // function instead (spec 5.3).
                    Err(Flow::Fail(err)) if !self.leaving => {
                        if let Some(def) = binding
                            .as_ref()
                            .and_then(|b| self.decl.get(&b.span.start).copied())
                        {
                            self.frame().vars.insert(def, err);
                        }
                        self.eval(handler)
                    }
                    other => other,
                }
            }
            ExprKind::Fail(x) => {
                let v = self.eval(x)?;
                self.leaving = true;
                Err(Flow::Fail(v))
            }
            ExprKind::Return(value) => {
                let v = match value {
                    Some(v) => self.eval(v)?,
                    None => Value::Unit,
                };
                Err(Flow::Return(v))
            }
            ExprKind::Break(label) => Err(Flow::Break(
                label.as_ref().map(|l| Rc::from(l.name.as_str())),
            )),
            ExprKind::Continue(label) => Err(Flow::Continue(
                label.as_ref().map(|l| Rc::from(l.name.as_str())),
            )),
            ExprKind::Spawn(kind, x, within) => self.spawn_within(*kind, x, within.as_deref()),
            ExprKind::Error => Err(self.panic("this code has a compile error", e.span)),
        }
    }

    pub(crate) fn eval_all(&mut self, items: &'a [Expr]) -> Result<Vec<Value<'a>>, Flow<'a>> {
        items.iter().map(|x| self.eval(x)).collect()
    }

    fn name(&mut self, id: &'a Ident, span: Span) -> Eval<'a> {
        let res = &self.c.resolution;
        let Some(&def) = res.uses.get(&id.span.start) else {
            // An ambiguous bare variant the checker settled from context.
            return Ok(Value::Unit);
        };
        match res.defs[def].kind {
            DefKind::Local { .. } | DefKind::Param => self
                .frames
                .last()
                .and_then(|f| f.vars.get(&def).cloned())
                .ok_or_else(|| self.panic(format!("`{}` has no value yet", id.name), span)),
            DefKind::Result => Ok(self.result.last().cloned().unwrap_or(Value::Unit)),
            DefKind::Field => Ok(self
                .invariant_fields
                .last()
                .and_then(|fs| fs.iter().find(|f| *f.name == *id.name))
                .map_or(Value::Unit, |f| f.value.clone())),
            DefKind::Fn => Ok(Value::Func(def)),
            // `count` in a `limit` rule: the window's calls, this one too.
            DefKind::Builtin if id.name == "count" => match self.totals.as_mut() {
                Some(t) => {
                    t.used = true;
                    Ok(Value::Int(t.count + 1, Prim::Int))
                }
                None => Err(self.panic("`count` is only for `limit` rules", span)),
            },
            // `tokens` in `limit Ai`: what the window's asks took, with
            // those under way counted as the settings say (spec 8.3).
            DefKind::Builtin if id.name == "tokens" => match self.totals.as_mut() {
                Some(t) => {
                    t.used = true;
                    t.tokens = true;
                    Ok(Value::Int(t.tokens_now, Prim::Int))
                }
                None => Err(self.panic("`tokens` is only for `limit Ai` rules", span)),
            },
            DefKind::Variant { of } => {
                let name = Rc::from(id.name.as_str());
                let life = self.variant_life(of, &name, &[]);
                Ok(Value::Variant {
                    union: of,
                    name,
                    fields: Vec::new(),
                    life,
                })
            }
            _ => Ok(Value::Unit),
        }
    }

    /// `Door.Open`: a variant named through its union.
    fn static_variant(&self, base: &Expr, name: &Ident) -> Option<Value<'a>> {
        let ExprKind::Name(id) = &base.kind else {
            return None;
        };
        let def = *self.c.resolution.uses.get(&id.span.start)?;
        let TypeBody::Union(vs) = &self.unions.get(&def)?.body else {
            return None;
        };
        vs.iter().any(|v| v.name.name == name.name).then(|| {
            let name = Rc::from(name.name.as_str());
            let life = self.variant_life(def, &name, &[]);
            Value::Variant {
                union: def,
                name,
                fields: Vec::new(),
                life,
            }
        })
    }

    fn closure(
        &mut self,
        params: Vec<Option<kavach_resolver::DefId>>,
        body: &'a Expr,
        dot: bool,
    ) -> Value<'a> {
        let frame = self.frames.last().expect("a frame");
        Value::Closure(Rc::new(Closure {
            params,
            body,
            dot,
            captured: frame.vars.clone(),
            generics: frame.generics.clone(),
            self_value: frame.self_value.clone(),
        }))
    }

    pub(crate) fn get_field(&self, v: &Value<'a>, name: &str, span: Span) -> Eval<'a> {
        match v {
            Value::Record { fields, .. } | Value::Variant { fields, .. } => fields
                .iter()
                .find(|f| *f.name == *name)
                .map(|f| f.value.clone())
                .ok_or_else(|| self.panic(format!("no field `{name}`"), span)),
            _ => Err(self.panic(format!("no field `{name}` on {}", v.display()), span)),
        }
    }

    pub(crate) fn index(&self, base: Value<'a>, index: Value<'a>, span: Span) -> Eval<'a> {
        match (base, index) {
            (Value::List(items), Value::Int(i, _)) => {
                match usize::try_from(i).ok().and_then(|i| items.get(i)) {
                    Some(v) => Ok(v.clone()),
                    None => {
                        Err(self.panic(kavach_runtime_core::list_index_panic(i, items.len()), span))
                    }
                }
            }
            (Value::Map(m), key) => Ok(m.get(&key).cloned().unwrap_or(Value::None)),
            (Value::Bytes(b), Value::Int(i, _)) => self.byte_at(&b, i, span),
            _ => Err(self.panic("this can't be indexed", span)),
        }
    }

    fn match_expr(&mut self, scrutinee: &'a Expr, arms: &'a [MatchArm], span: Span) -> Eval<'a> {
        let can_fail = arms
            .iter()
            .any(|a| matches!(a.pattern.kind, PatternKind::Ok(_) | PatternKind::Err(_)));
        let value = match self.eval(scrutinee) {
            Ok(v) if can_fail => Value::Ok(Box::new(v)),
            Err(Flow::Fail(e)) if can_fail => Value::Err(Box::new(e)),
            other => other?,
        };
        for arm in arms {
            let Some(bindings) = self.matches(&arm.pattern, &value)? else {
                continue;
            };
            for (def, v) in bindings {
                self.frame().vars.insert(def, v);
            }
            if let Some(g) = &arm.guard
                && self.eval(g)? != Value::Bool(true)
            {
                continue;
            }
            return self.eval(&arm.body);
        }
        Err(self.panic("no `match` arm matched", span))
    }
}
