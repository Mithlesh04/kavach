// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `on threads(n) for …` (spec 4.9) in the interpreter: each round a task
//! of its own, as `parallel_map` makes them (9.3), at most `n` at once.
//! A round starts with its sums at 0, and gives back its slot values and
//! sums, which are applied when it is waited for, in round order, so the
//! loop's result is the one-core result.

use crate::interp::Interp;
use crate::tasks::{TaskHandle, Work};
use crate::value::{Eval, Flow, Value};
use kavach_ast::rounds::{RoundWrites, round_writes};
use kavach_ast::{BinaryOp, Block, Expr, ExprKind, Pattern, PatternKind, Span};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A loop's writes, worked out once per loop: the slot targets, then the
/// sums', which a round reads back at its end.
pub(crate) struct Rounds {
    writes: RoundWrites,
    reads: Vec<Expr>,
}

/// Each loop's `Rounds` in a run, by the address of its body: one run's,
/// as tests run many programs in one process, where two loops can have
/// the same place in different files.
pub(crate) type Loops = Arc<Mutex<HashMap<usize, &'static Rounds>>>;

fn rounds_of(loops: Option<&Loops>, body: &Block, pattern: &Pattern) -> &'static Rounds {
    let key = body as *const Block as usize;
    let known = |l: &Loops| {
        l.lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&key)
            .copied()
    };
    if let Some(r) = loops.and_then(known) {
        return r;
    }
    // The round's slot: the loop's name, or the index of `(i, x)`.
    let slot = match &pattern.kind {
        PatternKind::Binding(id) => Some(id.name.as_str()),
        PatternKind::Tuple(ps) => match ps.first().map(|p| &p.kind) {
            Some(PatternKind::Binding(id)) => Some(id.name.as_str()),
            _ => None,
        },
        _ => None,
    };
    let writes = round_writes(body, slot);
    let reads = (writes.slots.iter().cloned())
        .chain(writes.sums.iter().map(|s| s.target.clone()))
        .collect();
    // Kept for the program's run: one per loop in the source.
    let r: &'static Rounds = Box::leak(Box::new(Rounds { writes, reads }));
    if let Some(l) = loops {
        l.lock().unwrap_or_else(|e| e.into_inner()).insert(key, r);
    }
    r
}

impl<'a> Interp<'a> {
    /// Runs the rounds over `items` as tasks, at most `n` at once (0: no
    /// limit). `None` when the body can leave early: then the caller runs
    /// the rounds in order.
    pub(crate) fn on_threads(
        &mut self,
        n: usize,
        pattern: &'a Pattern,
        items: Vec<Value<'a>>,
        body: &'a Block,
        span: Span,
    ) -> Result<Option<Value<'a>>, Flow<'a>> {
        let rounds = rounds_of(self.tasks.as_ref().map(|t| &t.loops), body, pattern);
        if rounds.writes.leaves {
            return Ok(None);
        }
        let mut started: Vec<(Arc<TaskHandle<'a>>, Value<'a>)> = Vec::new();
        let mut applied = 0;
        let all_rounds = self.tasks.as_ref().map(|t| Arc::clone(&t.rounds));
        let full = || all_rounds.as_ref().is_some_and(|r| r.full());
        for item in items {
            // At most `n` at once, and `[run] threads` in all the
            // program's loops; one with none running starts one (4.9).
            let running = started.len() - applied;
            if (n > 0 && running >= n) || (running > 0 && full()) {
                self.apply_round(rounds, pattern, &started[applied], span, &started)?;
                applied += 1;
            }
            self.bind_round(pattern, &item)?;
            // Each round adds into sums of its own, from 0.
            let mut saved = Vec::new();
            for s in &rounds.writes.sums {
                let v = self.eval(&s.target)?;
                self.assign(&s.target, zero_like(&v), span)?;
                saved.push(v);
            }
            let task = self.spawn_work(Work::Round(body, &rounds.reads), span)?;
            for (s, v) in rounds.writes.sums.iter().zip(saved) {
                self.assign(&s.target, v, span)?;
            }
            if let Some(r) = &all_rounds {
                r.add(1);
            }
            match task {
                Value::Task(t) => started.push((t, item)),
                _ => unreachable!("spawning gives a task"),
            }
        }
        for k in applied..started.len() {
            self.apply_round(rounds, pattern, &started[k], span, &started)?;
        }
        Ok(Some(Value::Unit))
    }

    /// `on cores(n) x` and `on cpu x` (spec 4.9): `x`, as where it runs
    /// can't change it; a count under 1 panics.
    pub(crate) fn placed(
        &mut self,
        place: &'a kavach_ast::ValuePlace,
        value: &'a Expr,
        span: Span,
    ) -> Eval<'a> {
        // `on gpu(n) x` (spec 15.6).
        if let kavach_ast::ValuePlace::Gpu(n) = place {
            return self.on_gpu(n, value);
        }
        if let kavach_ast::ValuePlace::Cores(Some(n)) = place
            && let Value::Int(k, _) = self.eval(n)?
            && k < 1
        {
            let why = kavach_runtime_core::place_count_panic(k as i64);
            return Err(self.panic(why, span));
        }
        self.eval(value)
    }

    fn bind_round(&mut self, pattern: &'a Pattern, item: &Value<'a>) -> Result<(), Flow<'a>> {
        if let Some(bindings) = self.matches(pattern, item)? {
            for (def, v) in bindings {
                self.frame().vars.insert(def, v);
            }
        }
        Ok(())
    }

    /// Waits for a round and applies what it gives: its slot values, and
    /// its sums added in.
    fn apply_round(
        &mut self,
        rounds: &'a Rounds,
        pattern: &'a Pattern,
        (task, item): &(Arc<TaskHandle<'a>>, Value<'a>),
        span: Span,
        all: &[(Arc<TaskHandle<'a>>, Value<'a>)],
    ) -> Result<(), Flow<'a>> {
        let waited = self.wait_task(task, None, span);
        if let Some(t) = &self.tasks {
            t.rounds.add(-1);
        }
        let values = match waited {
            Ok(Value::Tuple(values)) => values,
            Ok(_) => Vec::new(),
            Err(e) => {
                // The first error in round order is the loop's: the other
                // rounds' errors are no one else's to read.
                for (t, _) in all {
                    t.unread
                        .read
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                return Err(e);
            }
        };
        self.bind_round(pattern, item)?;
        let slots = rounds.writes.slots.len();
        for (target, v) in rounds.writes.slots.iter().zip(&values) {
            self.assign(target, v.clone(), span)?;
        }
        for (s, part) in rounds
            .writes
            .sums
            .iter()
            .zip(&values[slots.min(values.len())..])
        {
            let total = self.eval(&s.target)?;
            let mut sum = self.arith(BinaryOp::Add, total, part.clone(), span)?;
            if let Some(m) = s.modulo {
                let m = Value::Int(i128::from(m), kavach_checker::Prim::Int);
                sum = self.arith(BinaryOp::Rem, sum, m, span)?;
            }
            self.assign(&s.target, sum, span)?;
        }
        Ok(())
    }

    /// A round's body as a task, then the slot and sum values it ends with.
    pub(crate) fn run_round(&mut self, body: &'a Block, reads: &'a [Expr]) -> Eval<'a> {
        match self.block(body) {
            Ok(_) | Err(Flow::Continue(None)) => {}
            Err(e) => return Err(e),
        }
        let values = reads
            .iter()
            .map(|r| self.eval(r))
            .collect::<Result<_, _>>()?;
        Ok(Value::Tuple(values))
    }
}

/// 0 of a whole number's type.
fn zero_like<'a>(v: &Value<'a>) -> Value<'a> {
    match v {
        Value::Int(_, p) => Value::Int(0, *p),
        Value::BigInt(_) => Value::BigInt(std::rc::Rc::new(kavach_runtime_core::BigInt::from(0))),
        _ => Value::Int(0, kavach_checker::Prim::Int),
    }
}

/// Whether native code runs this loop's rounds as tasks too
/// (`compiler/kir/src/lower_threads.rs`): names, not other patterns, and
/// `(i, x)` only over `.enumerate()`. Other loops run in order.
pub(crate) fn as_tasks(pattern: &Pattern, iter: &Expr) -> bool {
    let plain = |p: &Pattern| matches!(p.kind, PatternKind::Binding(_) | PatternKind::Wildcard);
    match &pattern.kind {
        PatternKind::Tuple(ps) => {
            ps.len() == 2
                && ps.iter().all(plain)
                && matches!(&iter.kind, ExprKind::Call { callee, args, .. }
                    if args.is_empty() && matches!(&callee.kind, ExprKind::Field { name, .. } if name.name == "enumerate"))
        }
        _ => plain(pattern),
    }
}
