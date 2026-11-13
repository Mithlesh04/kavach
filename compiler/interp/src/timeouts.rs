//! `x.timeout(d)` (spec 9.6): a wait, or a network call, for at most `d`.
//! In a durable handler the limit's end is journaled (decision 0062).
//! And `spawn x within d`: a task that stops once `d` has passed.

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::{Expr, Span};
use std::time::Instant;

impl<'a> Interp<'a> {
    /// `x.timeout(d)` (spec 9.6), where `x` is `task.wait()`,
    /// `receive.take()` or `send.put(v)`: that wait, for at most `d`.
    /// `None` when `x` is none of them.
    pub(crate) fn timeout(
        &mut self,
        x: &'a Expr,
        args: &crate::calls::Args<'a>,
        span: Span,
    ) -> Option<Eval<'a>> {
        let kavach_ast::ExprKind::Call {
            callee,
            args: inner,
            ..
        } = &x.kind
        else {
            return None;
        };
        let kavach_ast::ExprKind::Field { base, name, .. } = &callee.kind else {
            return None;
        };
        if !matches!(
            name.name.as_str(),
            "wait"
                | "take"
                | "put"
                | "accept"
                | "read"
                | "write"
                | "send"
                | "receive"
                | "read_stream"
                | "write_stream"
                | "connect"
                | "connect_tls"
                | "connect_quic"
                | "start_tls"
                | "accept_tls"
        ) {
            return None;
        }
        let ms = args.first().map_or(0, |(_, v)| ms_of(v));
        let run = |me: &mut Self| -> Eval<'a> {
            let d = std::time::Duration::from_millis(ms as u64);
            if matches!(
                name.name.as_str(),
                "connect" | "connect_tls" | "connect_quic"
            ) {
                let mut values = Vec::new();
                for a in inner {
                    values.push(me.eval(&a.value)?);
                }
                return me.net_call(&name.name, &values, Some(d), span);
            }
            let target = me.eval(base)?;
            if let Value::Socket(s) = &target {
                let mut args = Vec::new();
                for a in inner {
                    args.push((a.name.as_ref().map(|n| n.name.as_str()), me.eval(&a.value)?));
                }
                let values = match name.name.as_str() {
                    "start_tls" | "accept_tls" => me.tls_args(&name.name, args),
                    _ => args.into_iter().map(|(_, v)| v).collect(),
                };
                return me.socket_method(s, &name.name, &values, Some(d), span);
            }
            if let Value::FakeConn(c) = &target {
                let mut values = Vec::new();
                for a in inner {
                    values.push(me.eval(&a.value)?);
                }
                return me.fake_conn_method(c, &name.name, &values);
            }
            // In a durable handler the limit's end is journaled, so after a
            // restart it ends when it would have (decision 0062).
            let now = crate::std_time::wall_ms();
            let due = me.journaled_time("time.timeout", now + ms as i128, span)?;
            let left = (due - now).clamp(0, ms as i128) as u64;
            let limit = Some((Instant::now() + std::time::Duration::from_millis(left), ms));
            match (target, name.name.as_str()) {
                (Value::Task(t), "wait") => me.wait_task(&t, limit, span),
                (Value::Channel(c, false), "take") => me.channel_take(&c, limit, span),
                (Value::Channel(c, true), "put") => {
                    let v = match inner.first() {
                        Some(a) => me.eval(&a.value)?,
                        None => Value::Unit,
                    };
                    me.channel_put(&c, v, limit, span)
                }
                _ => Err(me.panic("`timeout` here", span)),
            }
        };
        Some(run(self))
    }

    /// `spawn x within d` (spec 9.6): the task, which stops as if
    /// cancelled once `d` has passed.
    pub(crate) fn spawn_within(
        &mut self,
        kind: kavach_ast::SpawnKind,
        x: &'a Expr,
        within: Option<&'a Expr>,
    ) -> Eval<'a> {
        let task = match kind {
            kavach_ast::SpawnKind::Process => self.spawn_process(x)?,
            _ => self.spawn(x)?,
        };
        if let (Some(d), Value::Task(t)) = (within, &task) {
            let ms = ms_of(&self.eval(d)?);
            if let Some(tasks) = &self.tasks {
                tasks.sched.set_within(t.id, ms);
            }
        }
        Ok(task)
    }
}

/// The milliseconds of a `Duration`.
fn ms_of(d: &Value) -> i64 {
    match d {
        Value::Record { fields, .. } => match fields.first().map(|f| &f.value) {
            Some(Value::Int(ms, _)) => (*ms).max(0) as i64,
            _ => 0,
        },
        _ => 0,
    }
}
