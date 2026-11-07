//! Shrinking (spec 11.4): smaller versions of a value, tried in turn
//! until none of them still makes the function fail.

use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::Value;
use kavach_ast::{Span, TypeBody};
use kavach_runtime_core::Decimal;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// Values smaller than `v`, smallest first. Records that would break
    /// an invariant are left out.
    pub(crate) fn smaller(&mut self, v: &Value<'a>) -> Vec<Value<'a>> {
        let mut out = match v {
            Value::Int(n, p) if *n != 0 => {
                // 0, half, then ever closer to `n` (n/2, 3n/4, 7n/8, …, n∓1),
                // so shrinking finds the edge where the failure starts.
                let mut c = vec![0, n / 2];
                let mut step = n / 2;
                while step != 0 {
                    c.push(n - step);
                    step /= 2;
                }
                c.push(n - n.signum());
                c.dedup();
                c.into_iter().map(|m| Value::Int(m, *p)).collect()
            }
            Value::Float(f) if *f != 0.0 => vec![Value::Float(0.0), Value::Float(f.trunc())],
            Value::Decimal(d) if d.mantissa != 0 => {
                vec![
                    Value::Decimal(Decimal::from_int(0)),
                    Value::Decimal(Decimal::from_int(d.trunc())),
                ]
            }
            Value::Bool(true) => vec![Value::Bool(false)],
            Value::Char(c) if *c != 'a' => vec![Value::Char('a')],
            Value::Str(s) if !s.is_empty() => {
                let chars: Vec<char> = s.chars().collect();
                let text = |cs: &[char]| Value::Str(Rc::from(cs.iter().collect::<String>()));
                vec![
                    text(&[]),
                    text(&chars[..chars.len() / 2]),
                    text(&chars[1..]),
                    text(&chars[..chars.len() - 1]),
                ]
            }
            Value::List(items) if !items.is_empty() => {
                let list = |xs: Vec<Value<'a>>| Value::List(Rc::new(xs));
                let mut c = vec![list(Vec::new()), list(items[..items.len() / 2].to_vec())];
                for i in 0..items.len().min(8) {
                    let mut fewer = items.to_vec();
                    fewer.remove(i);
                    c.push(list(fewer));
                }
                for i in 0..items.len().min(8) {
                    for s in self.smaller(&items[i]) {
                        let mut changed = items.to_vec();
                        changed[i] = s;
                        c.push(list(changed));
                    }
                }
                c
            }
            Value::Map(m) if !m.is_empty() => {
                let mut c = vec![Value::Map(Rc::new(MapValue::default()))];
                for skip in 0..m.len().min(8) {
                    let mut fewer = MapValue::default();
                    for (i, (k, v)) in m.entries.iter().enumerate() {
                        if i != skip {
                            fewer.insert(k.clone(), v.clone());
                        }
                    }
                    c.push(Value::Map(Rc::new(fewer)));
                }
                c
            }
            Value::Tuple(items) => {
                let mut c = Vec::new();
                for i in 0..items.len() {
                    for s in self.smaller(&items[i]) {
                        let mut changed = items.clone();
                        changed[i] = s;
                        c.push(Value::Tuple(changed));
                    }
                }
                c
            }
            Value::Record {
                def, name, fields, ..
            } => {
                let mut c = Vec::new();
                for i in 0..fields.len() {
                    for s in self.smaller(&fields[i].value) {
                        let mut changed = fields.clone();
                        changed[i].value = s;
                        let value = Value::Record {
                            def: *def,
                            name: name.clone(),
                            fields: changed,
                            life: None,
                        };
                        if self.check_invariants(&value, Span::new(0, 0)).is_ok() {
                            c.push(value);
                        }
                    }
                }
                c
            }
            Value::Variant {
                union,
                name,
                fields,
            } => {
                // A variant without fields is the smallest value of a union.
                let mut c: Vec<Value<'a>> = match self.unions.get(union).map(|t| &t.body) {
                    Some(TypeBody::Union(vs)) if !fields.is_empty() => vs
                        .iter()
                        .filter(|v| v.fields.is_empty())
                        .map(|v| Value::Variant {
                            union: *union,
                            name: Rc::from(v.name.name.as_str()),
                            fields: Vec::new(),
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                for i in 0..fields.len() {
                    for s in self.smaller(&fields[i].value) {
                        let mut changed = fields.clone();
                        changed[i].value = s;
                        c.push(Value::Variant {
                            union: *union,
                            name: name.clone(),
                            fields: changed,
                        });
                    }
                }
                c
            }
            _ => Vec::new(),
        };
        // An optional that holds a value can also be `none`... but only
        // the caller knows the type, so `none` is offered by `shrink_args`.
        out.retain(|s| s != v);
        out
    }
}
