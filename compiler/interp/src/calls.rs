//! Calls: functions, closures, records, variants and methods.

use crate::interp::{Frame, Interp};
use crate::value::{Eval, Field, Flow, Value};
use kavach_ast::*;
use kavach_checker::Prim;
use kavach_resolver::{DefId, DefKind};
use std::rc::Rc;

pub(crate) type Args<'a> = Vec<(Option<&'a str>, Value<'a>)>;

/// Built-in methods that change their receiver, which must be a variable.
const MUTATING: &[&str] = &["push", "remove", "add"];

impl<'a> Interp<'a> {
    pub(crate) fn call(&mut self, e: &'a Expr, callee: &'a Expr, args: &'a [Arg]) -> Eval<'a> {
        let mut values: Args<'a> = Vec::new();
        for a in args {
            values.push((
                a.name.as_ref().map(|n| n.name.as_str()),
                self.eval(&a.value)?,
            ));
        }
        // `fs.read_text(path)`: a standard library call (spec 14).
        if let (ExprKind::Field { base, name, .. }, ExprKind::Call { generics, .. }) =
            (&callee.kind, &e.kind)
            && let ExprKind::Name(id) = &base.kind
        {
            let def = self.c.resolution.uses.get(&id.span.start);
            if let Some(module) = def.and_then(|d| self.std_modules.get(d)).cloned() {
                if module == "json" {
                    return self.json_call(&name.name, values, e);
                }
                let values = match (module.as_str(), name.name.as_str()) {
                    ("net", "connect_tls" | "listen_tls") => self.tls_args(&name.name, values),
                    // Named flags in their places; one left out is `Unit`.
                    ("base64", n) => {
                        let names: &[&str] = match n {
                            "encode" => &["data", "url", "pad"],
                            _ => &["text", "url"],
                        };
                        let placed = crate::json::arrange(values, names).into_iter();
                        placed.map(|v| v.unwrap_or(Value::Unit)).collect()
                    }
                    _ => values.into_iter().map(|(_, v)| v).collect(),
                };
                return self.std_call(&module, &name.name, generics, values, e.span);
            }
        }
        match &callee.kind {
            ExprKind::Name(id) => {
                if let Some(&def) = self.c.resolution.uses.get(&id.span.start) {
                    match self.c.resolution.defs[def].kind {
                        DefKind::Fn => {
                            let f = self.fns[&def];
                            if !f.generics.is_empty() {
                                self.next_generics = Some(self.type_args(f, e, args));
                            }
                            let v = self.call_fn(f, None, values, e.span)?;
                            self.write_back(args, e.span)?;
                            return Ok(v);
                        }
                        DefKind::Type => return self.construct(def, None, values, e.span),
                        DefKind::Variant { of } => {
                            return self.construct(of, Some(&id.name), values, e.span);
                        }
                        DefKind::Builtin => {
                            let name = self.c.resolution.defs[def].name.as_str();
                            if name == "ask" {
                                return self.ask(values, e);
                            }
                            return self.builtin_fn(name, values, e.span);
                        }
                        _ => {}
                    }
                }
            }
            ExprKind::Field { base, name, .. } => {
                self.inout_out.clear();
                let generics = match &e.kind {
                    ExprKind::Call { generics, .. } => generics.as_slice(),
                    _ => &[],
                };
                let v = self.method(e, base, name, generics, values)?;
                self.write_back(args, e.span)?;
                return Ok(v);
            }
            _ => {}
        }
        let f = self.eval(callee)?;
        self.call_value(f, values.into_iter().map(|(_, v)| v).collect(), e.span)
    }

    /// Calls a function value: a closure or a named function.
    pub(crate) fn call_value(
        &mut self,
        f: Value<'a>,
        args: Vec<Value<'a>>,
        span: Span,
    ) -> Eval<'a> {
        match f {
            Value::Closure(c) => {
                if self.frames.len() > 2_000 {
                    return Err(self.panic(
                        "stack overflow: calls are nested too deeply (spec 3.8)",
                        span,
                    ));
                }
                // A lambda runs with the type arguments of the call around
                // it, and of the function it was made in.
                let mut generics = self
                    .frames
                    .last()
                    .map(|f| f.generics.clone())
                    .unwrap_or_default();
                generics.extend(c.generics.iter().map(|(d, t)| (*d, t.clone())));
                let mut frame = Frame {
                    vars: c.captured.clone(),
                    self_value: c.self_value.clone(),
                    its: Vec::new(),
                    spawned: Vec::new(),
                    generics,
                };
                if c.dot {
                    frame.its.extend(args.into_iter().take(1));
                } else {
                    for (param, value) in c.params.iter().zip(args) {
                        if let Some(def) = param {
                            frame.vars.insert(*def, value);
                        }
                    }
                }
                self.frames.push(frame);
                let result = self.eval(c.body);
                let result = self.join_spawned(result, span);
                // A lambda's `try` and `fail` are its own (spec 3.3).
                self.leaving = false;
                self.frames.pop();
                match result {
                    Err(Flow::Return(v)) => Ok(v),
                    other => other,
                }
            }
            Value::Func(def) => {
                let f = self.fns[&def];
                self.call_fn(f, None, args.into_iter().map(|v| (None, v)).collect(), span)
            }
            other => Err(self.panic(format!("{} is not a function", other.display()), span)),
        }
    }

    /// Creates a record, or a variant of a union when `variant` is set.
    pub(crate) fn construct(
        &mut self,
        def: DefId,
        variant: Option<&str>,
        mut args: Args<'a>,
        span: Span,
    ) -> Eval<'a> {
        let decl = match variant {
            Some(_) => self.unions.get(&def),
            None => self.records.get(&def),
        };
        let Some(decl) = decl.copied() else {
            return Ok(Value::Unit);
        };
        let declared: &'a [kavach_ast::Field] = match (&decl.body, variant) {
            (TypeBody::Record(r), None) => &r.fields,
            (TypeBody::Union(vs), Some(v)) => vs
                .iter()
                .find(|x| x.name.name == v)
                .map_or(&[], |x| &x.fields),
            _ => &[],
        };
        let mut fields = Vec::new();
        for (i, f) in declared.iter().enumerate() {
            let given = args
                .iter()
                .position(|(n, _)| *n == Some(f.name.name.as_str()))
                .or((i < args.len() && args[i].0.is_none()).then_some(i));
            let value = match (given, &f.default) {
                (Some(j), _) => std::mem::replace(&mut args[j].1, Value::Unit),
                (None, Some(d)) => self.eval(d)?,
                (None, None) => Value::Unit,
            };
            fields.push(Field {
                name: Rc::from(f.name.name.as_str()),
                value,
                secret: f.is_secret,
            });
        }
        let value = match variant {
            Some(v) => Value::Variant {
                union: def,
                name: Rc::from(v),
                fields,
            },
            None => {
                let name: Rc<str> = Rc::from(decl.name.name.as_str());
                let life = self.life_for(def, &name, &fields);
                Value::Record {
                    def,
                    name,
                    fields,
                    life,
                }
            }
        };
        self.check_invariants(&value, span)?;
        Ok(value)
    }

    fn method(
        &mut self,
        call: &'a Expr,
        base: &'a Expr,
        name: &'a Ident,
        generics: &'a [kavach_ast::TypeExpr],
        args: Args<'a>,
    ) -> Eval<'a> {
        let span = call.span;
        if name.name == "timeout"
            && let Some(r) = self.timeout(base, &args, span)
        {
            return r;
        }
        if let ExprKind::Name(id) = &base.kind
            && let Some(&def) = self.c.resolution.uses.get(&id.span.start)
        {
            let d = &self.c.resolution.defs[def];
            match d.kind {
                DefKind::Type if self.unions.contains_key(&def) => {
                    // A type's own function first, then `all`, `parse`
                    // and `from` (spec 2.6), else a variant.
                    if let Some(f) = self
                        .methods
                        .get(&def)
                        .and_then(|m| m.get(name.name.as_str()))
                        .copied()
                    {
                        self.method_generics(f, call);
                        return self.call_fn(f, None, args, span);
                    }
                    if let Some(r) = self.enum_call(def, &name.name, args.first().map(|(_, v)| v)) {
                        return r;
                    }
                    return self.construct(def, Some(&name.name), args, span);
                }
                DefKind::Type => {
                    if let Some(f) = self
                        .methods
                        .get(&def)
                        .and_then(|m| m.get(name.name.as_str()))
                        .copied()
                    {
                        self.method_generics(f, call);
                        return self.call_fn(f, None, args, span);
                    }
                }
                DefKind::Builtin if d.name == "Float" && name.name == "from_bits" => {
                    let bits = match args.first() {
                        Some((_, Value::Int(n, _))) => *n as u64,
                        _ => 0,
                    };
                    return Ok(Value::Float(f64::from_bits(bits)));
                }
                DefKind::Builtin if d.name == "Set" && name.name == "of" => {
                    return Ok(self.set_of(args.into_iter().map(|(_, v)| v).collect()));
                }
                DefKind::Builtin if name.name == "with_capacity" && d.name == "Set" => {
                    return Ok(self.set_of(Vec::new()));
                }
                DefKind::Builtin if name.name == "with_capacity" && d.name == "Map" => {
                    return Ok(Value::Map(std::rc::Rc::default()));
                }
                DefKind::Agent if name.name == "start" => {
                    if let Some(decl) = self.agents.get(&def).copied() {
                        return self.start_agent(decl, args, span);
                    }
                }
                DefKind::Capability => {
                    if let Some(cap) = self.capabilities.get(&def).copied() {
                        return self.capability_call(cap, &name.name, args, span);
                    }
                }
                _ => {}
            }
        }
        // A user type's own `add` or `push` is a method like any other.
        let user_type = matches!(
            self.c.expr_types.get(&(base.span.start, base.span.end)),
            Some(kavach_checker::Ty::Adt(..) | kavach_checker::Ty::Param(_))
        );
        if MUTATING.contains(&name.name.as_str()) && !user_type {
            // A variable's list or map moves out while it changes, so
            // `items.push(x)` doesn't copy the whole list.
            let local = match &base.kind {
                ExprKind::Name(id) => self.c.resolution.uses.get(&id.span.start).copied(),
                _ => None,
            };
            let taken = local.and_then(|d| self.frame().vars.remove(&d));
            let receiver = match taken {
                Some(v) => v,
                None => self.eval(base)?,
            };
            let changed = self.mutate(receiver, &name.name, args, span)?;
            self.assign(base, changed, span)?;
            return Ok(Value::Unit);
        }
        let receiver = self.eval(base)?;
        if let Value::Agent(agent) = &receiver {
            return self.agent_event(agent.clone(), &name.name, args, span);
        }
        let user = match &receiver {
            Value::Record { def, .. } => self.methods.get(def),
            Value::Variant { union, .. } => self.methods.get(union),
            other => type_name(other).and_then(|t| self.ext_methods.get(t)),
        };
        if let Some(f) = user.and_then(|m| m.get(name.name.as_str())).copied() {
            self.method_generics(f, call);
            return self.call_fn(f, Some(receiver), args, span);
        }
        // A field holding a function: `h.on(x)` calls it (spec 3.4).
        if let Value::Record { fields, .. } = &receiver {
            let found = fields.iter().find(|f| &*f.name == name.name.as_str());
            if let Some(f) = found
                && matches!(f.value, Value::Func(_) | Value::Closure(_))
            {
                let value = f.value.clone();
                let args = args.into_iter().map(|(_, v)| v).collect();
                return self.call_value(value, args, span);
            }
        }
        if let Some(v) = self.enum_method(&receiver, &name.name) {
            return Ok(v);
        }
        if let (Value::Bytes(b), "as_list", [item]) = (&receiver, name.name.as_str(), generics) {
            return self.bytes_as_list(b, item, span);
        }
        let args = match (&receiver, name.name.as_str()) {
            (Value::Socket(_), "start_tls" | "accept_tls") => self.tls_args(&name.name, args),
            _ => args.into_iter().map(|(_, v)| v).collect(),
        };
        self.builtin_method(receiver, &name.name, args, span)
    }

    /// `Int(text)` and friends (spec 2.1): the text read whole, with
    /// spaces around it allowed, or a failure that says why.
    pub(crate) fn read_text(&self, prim: Prim, text: &str) -> Eval<'a> {
        let s = text.trim();
        let parsed = match prim {
            Prim::Float32 => s.parse::<f32>().ok().map(Value::Float32),
            Prim::Float => s.parse::<f64>().ok().map(Value::Float),
            Prim::Decimal => {
                kavach_runtime_core::AnyDecimal::parse(s).map(crate::bigint::decimal_value)
            }
            Prim::BigInt => s.parse().ok().map(|n| Value::BigInt(std::rc::Rc::new(n))),
            Prim::Bool => match s {
                "true" => Some(Value::Bool(true)),
                "false" => Some(Value::Bool(false)),
                _ => None,
            },
            Prim::Char => {
                let mut chars = text.chars();
                chars
                    .next()
                    .filter(|_| chars.next().is_none())
                    .map(Value::Char)
            }
            p => s
                .parse::<i128>()
                .ok()
                .and_then(|n| crate::ops::fits(n, p))
                .map(|n| Value::Int(n, p)),
        };
        parsed.ok_or_else(|| {
            Flow::Fail(Value::BuiltinError(
                format!("`{text}` is not a valid {}", prim.name()).into(),
            ))
        })
    }
}

/// The name `extend` uses for a built-in value's type.
fn type_name(v: &Value<'_>) -> Option<&'static str> {
    Some(match v {
        Value::Str(_) => "String",
        Value::Int(_, p) => p.name(),
        Value::Float(_) => "Float",
        Value::Float32(_) => "Float32",
        Value::Decimal(_) => "Decimal",
        Value::Bool(_) => "Bool",
        Value::Char(_) => "Char",
        _ => return None,
    })
}

impl<'a> Interp<'a> {
    /// After a call that returned: each `inout` parameter's last value
    /// into the place its argument names (spec 3.9).
    fn write_back(&mut self, args: &'a [Arg], span: Span) -> Result<(), Flow<'a>> {
        for (name, at, value) in std::mem::take(&mut self.inout_out) {
            let named = args
                .iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.name == name));
            let arg = named.or_else(|| args.iter().filter(|a| a.name.is_none()).nth(at));
            if let Some(a) = arg {
                self.assign(&a.value, value, span)?;
            }
        }
        Ok(())
    }
}
