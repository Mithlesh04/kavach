// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Numbers in types while the program runs (spec 2.2, 2.9): a value made
//! where its type has a `max` gets that limit, as `List.capacity(n)`
//! gives one; `try List<Int, max: 2>(xs)` fails with a `SizeError` when
//! `xs` doesn't fit; and a number parameter is the number its call or
//! value's type was given.

use crate::calls::Args;
use crate::interp::Interp;
use crate::items::Items;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_checker::{Prim, SizeKind, Ty};
use kavach_resolver::DefId;
use std::rc::Rc;

impl<'a> Interp<'a> {
    /// The number `t` stands for in the running call: a `Num`, or a
    /// number parameter's.
    pub(crate) fn number_of(&self, t: &Ty) -> Option<i64> {
        match self.concrete(t) {
            Ty::Num(n) => Some(n),
            _ => None,
        }
    }

    /// A number parameter's value (spec 2.9).
    pub(crate) fn number_param(&self, def: DefId) -> Option<Value<'a>> {
        let n = self.number_of(&Ty::Param(def))?;
        Some(Value::Int(i128::from(n), Prim::Int))
    }

    /// `value`, made at `span`, with the limit its type gives it, if any.
    pub(crate) fn limited(&mut self, span: Span, value: Value<'a>) -> Eval<'a> {
        let numbers = &self.c.numbers.limits;
        if numbers.is_empty() {
            return Ok(value);
        }
        let Some(n) = numbers.get(&(span.start, span.end)).cloned() else {
            return Ok(value);
        };
        let Some(n) = self.number_of(&n) else {
            return Ok(value);
        };
        let kind = match &value {
            Value::List(items) if items.ring => "Ring",
            Value::List(_) => "List",
            Value::Map(_) => "Dict",
            Value::Set(_) => "Set",
            _ => return Ok(value),
        };
        let first = match value {
            // `Set.capacity`'s first entries are a list.
            Value::Set(s) => {
                let values: Vec<Value<'a>> = s.entries.iter().map(|(k, _)| k.clone()).collect();
                Value::List(Rc::new(Items::new(values)))
            }
            v => v,
        };
        let n = Value::Int(i128::from(n), Prim::Int);
        self.capacity(kind, vec![(None, n), (None, first)], span)
    }

    /// `List<Int, max: 2>(xs)` and the other built-in types with numbers
    /// called with a value (spec 2.9), and `Ring<T, max: n>(items)`.
    /// `None` for any other call.
    pub(crate) fn sized_builtin(
        &mut self,
        name: &str,
        args: &mut Args<'a>,
        span: Span,
    ) -> Option<Eval<'a>> {
        if !matches!(name, "List" | "Dict" | "Set" | "Bytes" | "Ring") {
            return None;
        }
        let key = (span.start, span.end);
        if name == "Ring" {
            // The limit comes from the type, as for a literal (`limited`).
            let items = match args.pop() {
                Some((_, Value::List(xs))) => xs.to_vec(),
                _ => Vec::new(),
            };
            let mut ring = Items::new(items);
            ring.ring = true;
            return Some(Ok(Value::List(Rc::new(ring))));
        }
        let check = self.c.numbers.checks.get(&key).cloned();
        if check.is_none() && name == "Bytes" {
            return None;
        }
        let value = args.pop().map_or(Value::Unit, |(_, v)| v);
        let Some((kind, n, shown)) = check else {
            // A literal, checked while compiling.
            return Some(Ok(value));
        };
        let n = self.number_of(&n).unwrap_or(0);
        let (count, what) = match &value {
            Value::List(xs) => (xs.len(), "list"),
            Value::Map(m) => (m.len(), "dict"),
            Value::Set(s) => (s.len(), "set"),
            Value::Bytes(b) => (b.len(), "bytes"),
            _ => (0, "value"),
        };
        let fits = match kind {
            SizeKind::Max => count as i64 <= n,
            SizeKind::Len => count as i64 == n,
        };
        if fits {
            return Some(Ok(value));
        }
        let why =
            kavach_runtime_core::size_error(what, count, &shown.replace("{n}", &n.to_string()));
        Some(Err(Flow::Fail(Value::BuiltinError(why.into()))))
    }
}
