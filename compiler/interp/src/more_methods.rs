//! More methods of built-in types (spec 14.2): of text, lists, regular
//! expressions and sets, beyond those the interpreter runs itself.

use crate::interp::Interp;
use crate::map::MapValue;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::Prim;
use kavach_runtime_core::regex::Pattern;
use std::rc::Rc;

fn int<'v>(n: usize) -> Value<'v> {
    Value::Int(n as i128, Prim::Int)
}

impl<'a> Interp<'a> {
    /// More methods of built-in types (spec 14.2), or `None` if `name`
    /// isn't one of them.
    pub(crate) fn more_method(
        &mut self,
        recv: &Value<'a>,
        name: &str,
        args: &[Value<'a>],
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        if let Some(v) = self.number_method(recv, name, args, span)? {
            return Ok(Some(v));
        }
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Unit);
        Ok(Some(match (recv, name) {
            (Value::Str(s), _) => return Ok(self.more_text(s, name, args)),
            (Value::Regex(r), _) => return self.regex_method(r, name, args, span).map(Some),
            (Value::List(items), _) => return self.more_list(items, name, args, span),
            (Value::Set(set), _) => match name {
                "contains" => Value::Bool(set.get(&arg(0)).is_some()),
                "len" => int(set.len()),
                "is_empty" => Value::Bool(set.is_empty()),
                "is_full" => Value::Bool(set.is_full()),
                "to_list" => Value::List(Rc::new(
                    set.entries.iter().map(|(k, _)| k.clone()).collect(),
                )),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        }))
    }

    fn more_text(&self, s: &str, name: &str, args: &[Value<'a>]) -> Option<Value<'a>> {
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(t)) => t.to_string(),
            _ => String::new(),
        };
        let number = |i: usize| match args.get(i) {
            Some(Value::Int(n, _)) => (*n).max(0) as usize,
            _ => 0,
        };
        Some(match name {
            "replace" => self.str_value(&s.replace(&text(0), &text(1))),
            "lines" => Value::List(Rc::new(s.lines().map(|l| self.str_value(l)).collect())),
            "chars" => Value::List(Rc::new(s.chars().map(Value::Char).collect())),
            // `s.view()`: all of it; a view is its text here (spec 2.18).
            "view" | "to_string" if args.is_empty() => self.str_value(s),
            "index_of" | "find" => match s.find(&text(0)) {
                Some(byte) => int(s[..byte].chars().count()),
                None => Value::None,
            },
            "slice" | "view" => self.str_value(
                &s.chars()
                    .skip(number(0))
                    .take(number(1).saturating_sub(number(0)))
                    .collect::<String>(),
            ),
            _ => return None,
        })
    }

    fn more_list(
        &mut self,
        items: &Rc<crate::items::Items<'a>>,
        name: &str,
        args: &[Value<'a>],
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        let order = crate::ops::list_order;
        let arg = args.first().cloned().unwrap_or(Value::Unit);
        let list = |xs: Vec<Value<'a>>| Value::List(Rc::new(xs.into()));
        let count = |v: &Value<'a>| match v {
            Value::Int(n, _) => (*n).max(0) as usize,
            _ => 0,
        };
        Ok(Some(match name {
            "join" => {
                let parts: Vec<String> = items.iter().map(|v| v.display()).collect();
                self.str_value(&parts.join(&arg.display()))
            }
            "sort" => {
                let mut xs = items.to_vec();
                xs.sort_by(order);
                list(xs)
            }
            "sort_by" => {
                let mut keyed = Vec::new();
                for x in items.iter() {
                    keyed.push((
                        self.call_value(arg.clone(), vec![x.clone()], span)?,
                        x.clone(),
                    ));
                }
                keyed.sort_by(|a, b| order(&a.0, &b.0));
                list(keyed.into_iter().map(|(_, x)| x).collect())
            }
            "reverse" => list(items.iter().rev().cloned().collect()),
            "take" => list(items.iter().take(count(&arg)).cloned().collect()),
            // Items `start..end`, both clamped to the list (spec 14.2).
            "slice" => {
                let start = count(&arg).min(items.len());
                let end = count(args.get(1).unwrap_or(&Value::Unit)).clamp(start, items.len());
                list(items[start..end].to_vec())
            }
            "skip" => list(items.iter().skip(count(&arg)).cloned().collect()),
            "find" | "count" => {
                let mut found = Vec::new();
                for x in items.iter() {
                    if self.call_value(arg.clone(), vec![x.clone()], span)? == Value::Bool(true) {
                        found.push(x.clone());
                    }
                }
                match name {
                    "find" => found.into_iter().next().unwrap_or(Value::None),
                    _ => int(found.len()),
                }
            }
            "index_of" => items
                .iter()
                .position(|x| *x == arg)
                .map_or(Value::None, int),
            "min" => items
                .iter()
                .min_by(|a, b| order(a, b))
                .cloned()
                .unwrap_or(Value::None),
            "max" => items
                .iter()
                .max_by(|a, b| order(a, b))
                .cloned()
                .unwrap_or(Value::None),
            _ => return Ok(None),
        }))
    }

    /// A method of a compiled regular expression (spec 14.8).
    fn regex_method(&self, r: &Pattern, name: &str, args: &[Value<'a>], span: Span) -> Eval<'a> {
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let strs =
            |xs: Vec<String>| Value::List(Rc::new(xs.iter().map(|x| self.str_value(x)).collect()));
        let gave_up = |_| {
            self.panic(
                "regex gave up: this advanced pattern needed too much work on this text (spec 14.8)",
                span,
            )
        };
        let t = text(0);
        Ok(match name {
            "is_match" => Value::Bool(r.is_match(&t).map_err(gave_up)?),
            "find" => r
                .find(&t)
                .map_err(gave_up)?
                .map_or(Value::None, |m| self.str_value(&m)),
            "find_all" => strs(r.find_all(&t).map_err(gave_up)?),
            "split" => strs(r.split(&t).map_err(gave_up)?),
            "captures" => match r.captures(&t).map_err(gave_up)? {
                Some(groups) => Value::List(Rc::new(
                    groups
                        .iter()
                        .map(|g| g.as_ref().map_or(Value::None, |s| self.str_value(s)))
                        .collect(),
                )),
                None => Value::None,
            },
            "named_captures" => match r.named_captures(&t).map_err(gave_up)? {
                Some(pairs) => {
                    let mut map = MapValue::default();
                    for (k, v) in pairs {
                        map.insert(self.str_value(&k), self.str_value(&v));
                    }
                    Value::Map(Rc::new(map))
                }
                None => Value::None,
            },
            "replace" | "replace_all" => self.str_value(
                &r.replace(&t, &text(1), name == "replace_all")
                    .map_err(gave_up)?,
            ),
            _ => return Err(self.panic(format!("a regex has no method `{name}`"), span)),
        })
    }

    /// `Set.of(a, b, …)`.
    pub(crate) fn set_of(&self, values: Vec<Value<'a>>) -> Value<'a> {
        let mut set = MapValue::default();
        for v in values {
            set.insert(v, Value::Unit);
        }
        Value::Set(Rc::new(set))
    }
}
