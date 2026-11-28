// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Blocks, statements, assignment to places, and pattern matching.

use crate::interp::Interp;
use crate::value::{Eval, Field, Flow, Value};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::rc::Rc;

impl<'a> Interp<'a> {
    pub(crate) fn block(&mut self, b: &'a Block) -> Eval<'a> {
        self.block_lets.push(Vec::new());
        let result = self.statements(b);
        // The block's variables end here, the last made first (spec 6.1).
        let made = self.block_lets.pop().unwrap_or_default();
        for def in made.iter().rev() {
            self.frame().vars.remove(def);
        }
        match result {
            Err(e @ (Flow::Panic(..) | Flow::Exit(_))) => Err(e),
            // Cancelled: its `drop`s still run, with no stop point to
            // stop them (spec 9.6).
            Err(Flow::Cancelled) => {
                let was = std::mem::replace(&mut self.cleaning, true);
                let ran = self.run_drops(b.span);
                self.cleaning = was;
                ran?;
                Err(Flow::Cancelled)
            }
            result => {
                self.run_drops(b.span)?;
                result
            }
        }
    }

    fn statements(&mut self, b: &'a Block) -> Eval<'a> {
        let mut last = Value::Unit;
        for (i, s) in b.stmts.iter().enumerate() {
            // Temporaries of the statement before (or of a condition)
            // drop before this one runs.
            self.run_drops(s.span)?;
            let v = self.stmt(s)?;
            if i + 1 == b.stmts.len() && matches!(s.kind, StmtKind::Expr(_)) {
                last = v;
            }
            self.run_drops(s.span)?;
        }
        Ok(last)
    }

    pub(crate) fn stmt(&mut self, s: &'a Stmt) -> Eval<'a> {
        match &s.kind {
            StmtKind::Expr(e) => self.eval(e),
            StmtKind::Let { pattern, value, .. } => {
                let v = self.eval(value)?;
                if let Some(bindings) = self.matches(pattern, &v)? {
                    for (def, v) in bindings {
                        self.frame().vars.insert(def, v);
                        if let Some(made) = self.block_lets.last_mut() {
                            made.push(def);
                        }
                    }
                }
                Ok(Value::Unit)
            }
            StmtKind::Assign { target, op, value } => {
                self.assign_stmt(target, *op, value, s.span)?;
                Ok(Value::Unit)
            }
            // `on cores` and `on threads` (spec 4.9) give the one-core
            // result, so the interpreter runs the rounds in order.
            StmtKind::For {
                label,
                place,
                pattern,
                iter,
                body,
            } => {
                // `cores(n)`, `threads(n)`: worked out first, at least 1.
                let mut count = 0;
                if let Some(p) = place
                    && let Some(n) = &p.count
                    && let Value::Int(k, _) = self.eval(n)?
                {
                    if k < 1 {
                        let what = if p.threads { "thread" } else { "core" };
                        return Err(self.panic(
                            format!("a loop runs on at least one {what}, but the count is {k} (spec 4.9)"),
                            n.span,
                        ));
                    }
                    count = usize::try_from(k).unwrap_or(usize::MAX);
                }
                // A range is walked lazily: `0..n` never builds a list.
                // The rounds of `time.every_ms(ms)` (spec 14.4) each start
                // `ms` after the one before started, or at once if that
                // time has passed; other tasks run meanwhile.
                let mut ticker = None;
                let mut receiver = None;
                let mut reader = None;
                let looped = self.eval(iter)?;
                // `on threads`: each round a task (`on_threads.rs`).
                if place.as_ref().is_some_and(|p| p.threads)
                    && crate::on_threads::as_tasks(pattern, iter)
                    && (matches!(looped, Value::List(_))
                        || matches!(looped, Value::Range(..))
                            && matches!(iter.kind, ExprKind::Binary(..)))
                {
                    let items = match &looped {
                        Value::List(items) => items.iter().cloned().collect(),
                        Value::Range(a, b, k) => (*a..*b).map(|n| Value::Int(n, *k)).collect(),
                        _ => Vec::new(),
                    };
                    if let Some(done) = self.on_threads(count, pattern, items, body, s.span)? {
                        return Ok(done);
                    }
                }
                let items: Box<dyn Iterator<Item = Value<'a>>> = match looped {
                    // Values taken until the channel is closed and empty
                    // (spec 9.4).
                    Value::Channel(c, Some(_)) => {
                        receiver = Some(c);
                        Box::new(std::iter::repeat(Value::Unit))
                    }
                    // A file read as the loop goes (spec 14.3).
                    Value::Reader(r) => {
                        reader = Some(r);
                        Box::new(std::iter::repeat(Value::Unit))
                    }
                    Value::List(items) => {
                        Box::new(Rc::unwrap_or_clone(items).into_vec().into_iter())
                    }
                    Value::Range(a, b, k) => Box::new((a..b).map(move |n| Value::Int(n, k))),
                    Value::Ticker(ms) => {
                        let period = std::time::Duration::from_millis(ms);
                        ticker = Some((period, std::time::Instant::now() + period));
                        Box::new((0..).map(|n| Value::Int(n, kavach_checker::Prim::Int)))
                    }
                    Value::Set(s) => Box::new(
                        s.entries
                            .iter()
                            .map(|(k, _)| k.clone())
                            .collect::<Vec<_>>()
                            .into_iter(),
                    ),
                    Value::Map(m) => Box::new(
                        m.entries
                            .iter()
                            .map(|(k, v)| Value::Tuple(vec![k.clone(), v.clone()]))
                            .collect::<Vec<_>>()
                            .into_iter(),
                    ),
                    other => {
                        return Err(
                            self.panic(format!("can't loop over {}", other.display()), iter.span)
                        );
                    }
                };
                for item in items {
                    let item = match &receiver {
                        // Only the channel's end stops it: a sent `none` is a
                        // value (decision 0097).
                        Some(c) => match self.channel_next(c, None, s.span)? {
                            Some(v) => v,
                            None => break,
                        },
                        None => item,
                    };
                    let item = match &reader {
                        Some(r) => match self.reader_next(r, iter, s.span)? {
                            Some(v) => v,
                            None => break,
                        },
                        None => item,
                    };
                    // A round's wait is a `sleep` until its start, so in a
                    // durable handler it carries on across a restart.
                    if let Some((period, next)) = &mut ticker {
                        self.journaled_sleep(*next, s.span)?;
                        *next = std::time::Instant::now() + *period;
                    }
                    self.step(s.span)?;
                    if let Some(bindings) = self.matches(pattern, &item)? {
                        for (def, v) in bindings {
                            self.frame().vars.insert(def, v);
                        }
                    }
                    match self.block(body) {
                        Ok(_) => {}
                        Err(Flow::Continue(l)) if aimed_at(&l, label) => {}
                        Err(Flow::Break(l)) if aimed_at(&l, label) => break,
                        Err(other) => return Err(other),
                    }
                }
                Ok(Value::Unit)
            }
            StmtKind::While { label, cond, body } => {
                while self.eval(cond)? == Value::Bool(true) {
                    self.step(s.span)?;
                    match self.block(body) {
                        Ok(_) => {}
                        Err(Flow::Continue(l)) if aimed_at(&l, label) => {}
                        Err(Flow::Break(l)) if aimed_at(&l, label) => break,
                        Err(other) => return Err(other),
                    }
                }
                Ok(Value::Unit)
            }
            StmtKind::Assert(e) => self.assert_stmt(e),
        }
    }

    /// Stores each part of tuple `value` into its place of `target`, left
    /// to right, a tuple of places too (spec 1.14).
    pub(crate) fn assign_parts(
        &mut self,
        target: &'a Expr,
        value: Value<'a>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        match (&target.kind, value) {
            (ExprKind::Tuple(places), Value::Tuple(parts)) => {
                for (place, part) in places.iter().zip(parts) {
                    self.assign_parts(place, part, span)?;
                }
                Ok(())
            }
            (_, value) => self.assign(target, value, span),
        }
    }

    /// The bindings `p` makes when it matches `v`, or `None` if it doesn't.
    pub(crate) fn matches(
        &mut self,
        p: &'a Pattern,
        v: &Value<'a>,
    ) -> Result<Option<Vec<(DefId, Value<'a>)>>, Flow<'a>> {
        let mut out = Vec::new();
        let ok = self.match_into(p, v, &mut out)?;
        Ok(ok.then_some(out))
    }

    pub(crate) fn match_into(
        &mut self,
        p: &'a Pattern,
        v: &Value<'a>,
        out: &mut Vec<(DefId, Value<'a>)>,
    ) -> Result<bool, Flow<'a>> {
        Ok(match (&p.kind, v) {
            (PatternKind::Wildcard, _) => true,
            (PatternKind::Binding(id), _) => {
                if let Some(&def) = self.decl.get(&id.span.start) {
                    out.push((def, v.clone()));
                }
                true
            }
            (PatternKind::Literal(e), _) => self.eval(e)? == *v,
            (PatternKind::Range { lo, hi, inclusive }, Value::Int(n, _)) => {
                let (Value::Int(a, _), Value::Int(b, _)) = (self.eval(lo)?, self.eval(hi)?) else {
                    return Ok(false);
                };
                *n >= a && if *inclusive { *n <= b } else { *n < b }
            }
            (PatternKind::Tuple(ps), Value::Tuple(items)) if ps.len() == items.len() => {
                for (p, item) in ps.iter().zip(items) {
                    if !self.match_into(p, item, out)? {
                        return Ok(false);
                    }
                }
                true
            }
            // Destructuring (spec 1.12): lists by position, maps by key,
            // records by field (`destructure.rs`).
            (PatternKind::List { items, rest }, Value::List(xs)) => {
                for (i, it) in items.iter().enumerate() {
                    self.take_part(&it.pattern, xs.get(i).cloned(), it.default.as_ref(), out)?;
                }
                if let Some(def) = rest.as_ref().and_then(|r| self.decl.get(&r.span.start)) {
                    let left = xs.iter().skip(items.len()).cloned().collect();
                    out.push((*def, Value::List(Rc::new(left))));
                }
                true
            }
            (PatternKind::List { items, .. }, Value::Map(m)) => {
                for it in items {
                    let Some(key) = self.map_key(it)? else {
                        continue;
                    };
                    self.take_part(&it.pattern, m.get(&key).cloned(), it.default.as_ref(), out)?;
                }
                true
            }
            (PatternKind::Record(fields), Value::Record { fields: values, .. }) => {
                for f in fields {
                    let part = values
                        .iter()
                        .find(|x| *x.name == *f.field.name)
                        .map(|x| x.value.clone());
                    self.take_part(&f.pattern, part, f.default.as_ref(), out)?;
                }
                true
            }
            (PatternKind::None, Value::None) => true,
            (PatternKind::Ok(inner), Value::Ok(x)) | (PatternKind::Err(inner), Value::Err(x)) => {
                let mut x = (**x).clone();
                while let Value::Context(_, inner_error) = x {
                    x = *inner_error;
                }
                self.match_into(inner, &x, out)?
            }
            // An `Error` with `context` added still matches the error it
            // holds (spec 5.5, 5.7).
            (PatternKind::Variant { .. }, Value::Context(_, inner)) => {
                self.match_into(p, inner, out)?
            }
            (
                PatternKind::Variant { path, fields },
                Value::Variant {
                    union,
                    name,
                    fields: values,
                    ..
                },
            ) => {
                let last = path.segments.last().expect("a name");
                let res = &self.c.resolution;
                let kind = res.uses.get(&last.span.start).map(|d| res.defs[*d].kind);
                // An ambiguous bare name was settled by the checker from the
                // value's type, so its name is enough here.
                let is_variant = matches!(kind, Some(DefKind::Variant { .. }))
                    || res.ambiguous.contains_key(&last.span.start);
                // A name with its type (`FsError.Denied`) matches only that
                // type: an `Error` can hold another one's `Denied`.
                let other_type = path.segments.len() > 1
                    && matches!(kind, Some(DefKind::Variant { of }) if of != *union);
                if !is_variant || other_type || **name != *last.name {
                    return Ok(false);
                }
                self.match_fields(fields.as_deref(), values, out)?
            }
            // A record pattern (spec 4.3): every value of its type fits,
            // so only the patterns inside its fields can fail.
            (
                PatternKind::Variant { path, fields },
                Value::Record {
                    def,
                    fields: values,
                    ..
                },
            ) => {
                let last = path.segments.last().expect("a name");
                if self.c.resolution.uses.get(&last.span.start) != Some(def) {
                    return Ok(false);
                }
                self.match_fields(fields.as_deref(), values, out)?
            }
            _ => false,
        })
    }

    /// Whether the fields of a variant or record match their patterns: a
    /// bare name takes the field it names, else the one at its place.
    fn match_fields(
        &mut self,
        fields: Option<&'a [FieldPattern]>,
        values: &[Field<'a>],
        out: &mut Vec<(DefId, Value<'a>)>,
    ) -> Result<bool, Flow<'a>> {
        for (i, fp) in fields.unwrap_or_default().iter().enumerate() {
            let field_name = match (&fp.name, &fp.pattern.kind) {
                (Some(n), _) => Some(n.name.as_str()),
                (None, PatternKind::Binding(b)) if values.iter().any(|f| *f.name == *b.name) => {
                    Some(b.name.as_str())
                }
                _ => None,
            };
            let value = match field_name {
                Some(n) => values.iter().find(|f| *f.name == *n),
                None => values.get(i),
            };
            match value {
                Some(f) if self.match_into(&fp.pattern, &f.value, out)? => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}

/// Whether a `break` or `continue` naming `named` (none: the innermost
/// loop) is for the loop labeled `label` (spec 4.6).
fn aimed_at(named: &Option<Rc<str>>, label: &Option<kavach_ast::Ident>) -> bool {
    match (named, label) {
        (None, _) => true,
        (Some(n), Some(l)) => **n == *l.name,
        (Some(_), None) => false,
    }
}
