// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Prelude functions (spec 10.2) and methods of built-in types.

use crate::calls::Args;
use crate::interp::Interp;
use crate::ops::{fits, int_range};
use crate::value::{Eval, Flow, Value};
use kavach_ast::{BinaryOp, Span};
use kavach_checker::Prim;
use kavach_runtime_core::text_methods as tm;
use std::cell::RefCell;
use std::rc::Rc;

impl<'a> Interp<'a> {
    pub(crate) fn builtin_fn(&mut self, name: &str, mut args: Args<'a>, span: Span) -> Eval<'a> {
        // `try List<Int, max: 2>(xs)`, `Ring<T, max: n>(items)` (2.9).
        if let Some(made) = self.sized_builtin(name, &mut args, span) {
            return made;
        }
        if name == "Bytes" {
            let list = args.into_iter().next().map_or(Value::Unit, |(_, v)| v);
            return Ok(self.bytes_of(list));
        }
        if name == "TextBuilder" {
            return Ok(self.text_builder(args.into_iter().map(|(_, v)| v).collect()));
        }
        // `Error("disk full")`: an error that is only its message (spec 5.5).
        if name == "Error" {
            return Ok(args.into_iter().next().map_or(Value::Unit, |(_, v)| v));
        }
        if name == "Channel" {
            let capacity = args.into_iter().next().map_or(Value::Unit, |(_, v)| v);
            return self.new_channel(capacity, span);
        }
        if name == "random_bits" {
            let bits = self.random_bits();
            return Ok(Value::Int(bits as i64 as i128, Prim::Int));
        }
        if name == "random_ints" {
            return Ok(self.random_ints(&args));
        }
        if name == "test_random" {
            let span_of = match args.first() {
                Some((_, Value::Int(n, _))) => (*n).max(0),
                _ => 0,
            };
            let n = self.gen_rng.range(0, span_of);
            return Ok(Value::Int(n, kavach_checker::Prim::Int));
        }
        if name == "log_write" {
            let mut values = args.into_iter().map(|(_, v)| v);
            let level = match values.next() {
                Some(Value::Int(n, _)) => n as i64,
                _ => 1,
            };
            let line = values.next().map_or(String::new(), |v| v.display());
            let event = values.next().unwrap_or(Value::Unit);
            self.log_write(level, &line, &event);
            return Ok(Value::Unit);
        }
        let arg = args
            .into_iter()
            .next()
            .map(|(_, v)| v)
            .unwrap_or(Value::Unit);
        match name {
            "print" => {
                let text = self.shown(&arg, true, span)?;
                self.out.push_str(&text);
                self.out.push('\n');
                self.show_live();
                Ok(Value::Unit)
            }
            "debug" => {
                let (path, line) = self.c.shown_location(span.start);
                let shown = format!("[{path}:{line}] {}\n", arg.display());
                self.err.push_str(&shown);
                Ok(Value::Unit)
            }
            "panic" => Err(self.panic(arg.display(), span)),
            // `sum(amount)` in a `limit` rule: the window's total with this
            // call's amount added (spec 8.3).
            "sum" => {
                let Some(totals) = self.totals.as_mut() else {
                    return Err(self.panic("`sum` is only for `limit` rules", span));
                };
                totals.added = Some(arg.clone());
                totals.used = true;
                match totals.sum.clone() {
                    Some(before) => self.arith(BinaryOp::Add, before, arg, span),
                    None => Ok(arg),
                }
            }
            "reveal" => Ok(arg),
            "Shared" => {
                let v = Rc::new(RefCell::new(arg));
                if let Some(tasks) = &self.tasks {
                    tasks.made_shared(&v, span.start);
                }
                Ok(Value::Shared(v))
            }
            "env" => match self.env_value(&arg.display()) {
                Some(v) => Ok(self.str_value(&v)),
                None => Err(Flow::Fail(Value::BuiltinError(
                    format!("environment variable `{}` is not set", arg.display()).into(),
                ))),
            },
            "alloc" | "free" => Err(self.panic(
                "`unsafe` code runs with the native compiler, not the interpreter",
                span,
            )),
            _ => self.convert(name, arg, span),
        }
    }

    /// `list.push(x)` and `map.insert(k, v)`: returns the changed receiver.
    pub(crate) fn mutate(
        &mut self,
        receiver: Value<'a>,
        name: &str,
        args: Args<'a>,
        span: Span,
    ) -> Eval<'a> {
        let mut args = args.into_iter().map(|(_, v)| v);
        match (receiver, name) {
            (Value::List(mut items), "push") => {
                let item = args.next().unwrap_or(Value::Unit);
                crate::items::push(&mut items, item).map_err(|m| self.panic(m, span))?;
                Ok(Value::List(items))
            }
            (Value::List(items), "swap") => self.list_swap(items, args, span),
            (Value::Set(mut set), "add") => {
                let item = args.next().unwrap_or(Value::Unit);
                set.room_for(&item, "set")
                    .map_err(|m| self.panic(m, span))?;
                Rc::make_mut(&mut set).insert(item, Value::Unit);
                Ok(Value::Set(set))
            }
            (Value::Set(mut set), "remove") => {
                Rc::make_mut(&mut set).remove(&args.next().unwrap_or(Value::Unit));
                Ok(Value::Set(set))
            }
            (Value::Map(mut m), "remove") => {
                Rc::make_mut(&mut m).remove(&args.next().unwrap_or(Value::Unit));
                Ok(Value::Map(m))
            }
            // A `TextBuilder` (spec 2.18).
            (Value::Str(s), _) => self.builder_change(s, name, args.collect(), span),
            (other, _) => {
                Err(self.panic(format!("{} has no method `{name}`", other.display()), span))
            }
        }
    }

    pub(crate) fn builtin_method(
        &mut self,
        recv: Value<'a>,
        name: &str,
        args: Vec<Value<'a>>,
        span: Span,
    ) -> Eval<'a> {
        // A range is the list of its numbers to the list methods (spec 2.2).
        let recv = match recv {
            Value::Range(a, b, k) => {
                let items: Vec<Value<'a>> = (a..b).map(|n| Value::Int(n, k)).collect();
                if name == "to_list" {
                    return Ok(Value::List(Rc::new(items.into())));
                }
                Value::List(Rc::new(items.into()))
            }
            r => r,
        };
        // A present optional is the value itself (spec 2.7), so these come
        // first, whatever kind of value it holds: only optionals have them.
        if args.is_empty() && !matches!(recv, Value::None) {
            match name {
                "unwrap" => return Ok(recv),
                "is_some" | "is_none" => return Ok(Value::Bool(name == "is_some")),
                _ => {}
            }
        }
        // `task.wait()` (spec 9.1).
        if let Value::Socket(s) = &recv {
            return self.socket_method(s, name, &args, None, span);
        }
        if let Value::FakeConn(c) = &recv {
            return self.fake_conn_method(c, name, &args);
        }
        if let Value::Bytes(b) = &recv {
            return self.bytes_method(b, name, &args, span);
        }
        if let (Value::Str(s), "bytes") = (&recv, name) {
            return Ok(Value::Bytes(s.as_bytes().into()));
        }
        if let (Value::Task(t), "wait") = (&recv, name) {
            return self.wait_task(t, None, span);
        }
        // `task.cancel()` (spec 9.6): it stops at once, even computing.
        if let (Value::Task(t), "cancel") = (&recv, name) {
            if let Some(tasks) = &self.tasks {
                tasks.sched.cancel_now(t.id);
            }
            return Ok(Value::Unit);
        }
        // Channels (spec 9.4).
        match (&recv, name) {
            (Value::Channel(c, None), "put") => {
                let v = args.into_iter().next().unwrap_or(Value::Unit);
                return self.channel_put(c, v, None, span);
            }
            (Value::Channel(c, None), "try_put") => {
                let v = args.into_iter().next().unwrap_or(Value::Unit);
                return Ok(Value::Bool(self.channel_try_put(c, v)));
            }
            (Value::Channel(c, None), "close") => return self.channel_close(c),
            (Value::Channel(c, Some(_)), "take") => return self.channel_take(c, None, span),
            _ => {}
        }
        // A `Char`'s kind checks (spec 14.2, *Characters*).
        if let Value::Char(c) = &recv
            && let Some(i) = tm::CHAR_CHECKS.iter().position(|n| *n == name)
        {
            return Ok(Value::Bool(tm::char_check(*c, i)));
        }
        // Text methods that came from `std.text` (spec 14.2).
        if let Value::Str(s) = &recv
            && let Some(v) = self.text_method(&s.clone(), name, &args, span)
        {
            return v;
        }
        if let Some(v) = self.more_method(&recv, name, &args, span)? {
            return Ok(v);
        }
        let int = |n: usize| Value::Int(n as i128, Prim::Int);
        let arg = args.first().cloned().unwrap_or(Value::Unit);
        match (recv, name) {
            (Value::List(items), "parallel_map") => {
                self.parallel_map(&items, arg, args.get(1).cloned(), span)
            }
            (Value::List(items), _) => self.list_method(items, name, arg, span),
            (Value::Map(m), "len") => Ok(int(m.len())),
            (Value::Map(m), "is_empty") => Ok(Value::Bool(m.is_empty())),
            (Value::Map(m), "is_full") => Ok(Value::Bool(m.is_full())),
            (Value::Map(m), "get") => Ok(m.get(&arg).cloned().unwrap_or(Value::None)),
            (Value::Map(m), "contains_key") => Ok(Value::Bool(m.get(&arg).is_some())),
            (Value::Map(m), "keys") => Ok(Value::List(Rc::new(
                m.entries.iter().map(|(k, _)| k.clone()).collect(),
            ))),
            (Value::Map(m), "values") => Ok(Value::List(Rc::new(
                m.entries.iter().map(|(_, v)| v.clone()).collect(),
            ))),
            (Value::Str(s), _) => self.string_method(&s, name, &arg, span),
            (Value::Int(n, p), _) => {
                let Value::Int(m, _) = arg else {
                    return Err(self.panic("expected an integer", span));
                };
                let exact = match name {
                    "wrapping_add" | "checked_add" => n.checked_add(m),
                    "wrapping_sub" | "checked_sub" => n.checked_sub(m),
                    "wrapping_mul" | "checked_mul" => n.checked_mul(m),
                    "checked_div" if m == 0 => None,
                    "checked_div" => n.checked_div(m),
                    _ => {
                        return Err(
                            self.panic(format!("{} has no method `{name}`", p.name()), span)
                        );
                    }
                };
                if name.starts_with("wrapping") {
                    let (lo, hi) = int_range(p);
                    let wrapped = exact.map_or(0, |x| (x - lo).rem_euclid(hi - lo + 1) + lo);
                    return Ok(Value::Int(wrapped, p));
                }
                Ok(exact
                    .and_then(|x| fits(x, p))
                    .map_or(Value::None, |x| Value::Int(x, p)))
            }
            (Value::None, "unwrap") => Err(self.panic("`.unwrap()` on `none`", span)),
            (Value::None, "is_none") => Ok(Value::Bool(true)),
            (Value::None, "is_some") => Ok(Value::Bool(false)),
            (Value::Shared(cell), "get") => Ok(cell.borrow().clone()),
            (Value::Shared(cell), "weak") => Ok(Value::Weak(Rc::downgrade(&cell))),
            (Value::Weak(cell), "shared") => Ok(cell.upgrade().map_or(Value::None, Value::Shared)),
            (Value::Shared(cell), "update" | "update_with") => {
                let current = cell.borrow().clone();
                // Held while the function runs: it may not wait (6.2).
                kavach_runtime_core::tasks::hold(true);
                let next = self.call_value(arg, vec![current], span);
                kavach_runtime_core::tasks::hold(false);
                // `update` gives the new value; `update_with` the second
                // of the pair its function gives.
                let (next, result) = match (name, next?) {
                    ("update", next) => (next.clone(), next),
                    (_, Value::Tuple(pair)) if pair.len() == 2 => {
                        (pair[0].clone(), pair[1].clone())
                    }
                    (_, other) => (other, Value::Unit),
                };
                *cell.borrow_mut() = next;
                Ok(result)
            }
            // `spawn` runs at once, so a task's value is ready.
            (v, "wait") => Ok(v),
            (other, _) => {
                Err(self.panic(format!("{} has no method `{name}`", other.display()), span))
            }
        }
    }

    fn list_method(
        &mut self,
        items: Rc<crate::items::Items<'a>>,
        name: &str,
        arg: Value<'a>,
        span: Span,
    ) -> Eval<'a> {
        let int = |n: usize| Value::Int(n as i128, Prim::Int);
        Ok(match name {
            "len" => int(items.len()),
            "is_empty" => Value::Bool(items.is_empty()),
            "is_full" => Value::Bool(items.is_full()),
            // A ring's items as a list, with no limit (spec 2.2).
            "to_list" => Value::List(Rc::new(items.to_vec().into())),
            "get" => match arg {
                Value::Int(i, _) => usize::try_from(i)
                    .ok()
                    .and_then(|i| items.get(i).cloned())
                    .unwrap_or(Value::None),
                _ => Value::None,
            },
            "first" => items.first().cloned().unwrap_or(Value::None),
            "last" => items.last().cloned().unwrap_or(Value::None),
            "contains" => Value::Bool(items.contains(&arg)),
            "enumerate" => Value::List(Rc::new(
                items
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, v)| Value::Tuple(vec![int(i), v]))
                    .collect(),
            )),
            "filter" | "map" | "any" | "all" | "flat_map" | "sum" => {
                let mut mapped = Vec::new();
                for item in items.iter() {
                    mapped.push(self.call_value(arg.clone(), vec![item.clone()], span)?);
                }
                match name {
                    "filter" => Value::List(Rc::new(
                        items
                            .iter()
                            .zip(&mapped)
                            .filter(|(_, keep)| **keep == Value::Bool(true))
                            .map(|(v, _)| v.clone())
                            .collect(),
                    )),
                    "map" => Value::List(Rc::new(mapped.into())),
                    "any" => Value::Bool(mapped.contains(&Value::Bool(true))),
                    "all" => Value::Bool(mapped.iter().all(|v| *v == Value::Bool(true))),
                    "flat_map" => Value::List(Rc::new(
                        mapped
                            .into_iter()
                            .flat_map(|v| match v {
                                Value::List(xs) => Rc::unwrap_or_clone(xs).into_vec(),
                                v => vec![v],
                            })
                            .collect(),
                    )),
                    _ => {
                        let mut values = mapped.into_iter();
                        // An empty sum is the zero of the call's type: `0.0`
                        // for a `Float` lambda, as native code gives.
                        let zero = || match self.c.expr_types.get(&(span.start, span.end)) {
                            Some(kavach_checker::Ty::Prim(Prim::Float)) => Value::Float(0.0),
                            Some(kavach_checker::Ty::Prim(Prim::Float32)) => Value::Float32(0.0),
                            Some(kavach_checker::Ty::Prim(Prim::Decimal)) => {
                                crate::bigint::decimal_value(
                                    kavach_runtime_core::AnyDecimal::parse("0")
                                        .expect("0 is a number"),
                                )
                            }
                            Some(kavach_checker::Ty::Prim(p)) if p.is_int() => Value::Int(0, *p),
                            _ => Value::Int(0, Prim::Int),
                        };
                        let mut total = values.next().unwrap_or_else(zero);
                        for v in values {
                            total = self.arith(BinaryOp::Add, total, v, span)?;
                        }
                        total
                    }
                }
            }
            _ => return Err(self.panic(format!("lists have no method `{name}`"), span)),
        })
    }

    fn string_method(&mut self, s: &str, name: &str, arg: &Value<'a>, span: Span) -> Eval<'a> {
        let text = || arg.display();
        Ok(match name {
            "len" => Value::Int(s.chars().count() as i128, Prim::Int),
            "is_empty" => Value::Bool(s.is_empty()),
            "upper" => self.str_value(&s.to_uppercase()),
            "lower" => self.str_value(&s.to_lowercase()),
            "trim" => self.str_value(s.trim()),
            "contains" => Value::Bool(
                kavach_runtime_core::files::find_bytes(s.as_bytes(), text().as_bytes()).is_some(),
            ),
            "starts_with" => Value::Bool(s.starts_with(&text())),
            "compare" => Value::Int(s.cmp(text().as_str()) as i8 as i128, Prim::Int),
            "ends_with" => Value::Bool(s.ends_with(&text())),
            "split" => {
                let sep = text();
                if sep.is_empty() {
                    return Err(self.panic("`split(\"\")` has no separator to split at: use `chars()` for the characters (spec 14.2)", span));
                }
                Value::List(Rc::new(s.split(&sep).map(|p| self.str_value(p)).collect()))
            }
            "repeat" => match arg {
                Value::Int(n, _) => {
                    let n = usize::try_from(*n).unwrap_or(0);
                    // Big text costs time: one step of the test budget per
                    // kilobyte (spec 11.9).
                    if self.step_limit.is_some() {
                        let kilobytes = s.len().saturating_mul(n) / 1024;
                        self.steps = self.steps.saturating_add(kilobytes as u64);
                        self.step(span)?;
                    }
                    match kavach_runtime_core::repeat(s, n) {
                        Ok(text) => self.str_value(&text),
                        Err(message) => return Err(self.panic(message, span)),
                    }
                }
                _ => self.str_value(""),
            },
            _ => return Err(self.panic(format!("text has no method `{name}`"), span)),
        })
    }
}

impl Interp<'_> {
    /// Shows what was printed now, when output is live (`kavach run`).
    pub(crate) fn show_live(&mut self) {
        if self.tasks.as_ref().is_some_and(|t| t.live) {
            self.flush();
        }
    }
}
