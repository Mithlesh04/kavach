//! `std.time` (spec 14.4): clocks, sleeping, intervals and timers. A wait
//! lets other tasks run (`tasks.rs`).

use crate::interp::Interp;
use crate::value::{Eval, Value};
use kavach_ast::{Expr, Span};
use kavach_checker::Prim;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

/// A test's frozen clock (spec 14.4): what `now_ms` and `monotonic_ms`
/// give, moved on only by sleeping.
pub(crate) type Frozen = Option<Rc<Cell<(i128, i128)>>>;

/// Milliseconds since 1970-01-01 UTC, by the wall clock.
pub(crate) fn wall_ms() -> i128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i128)
}

/// The milliseconds of a `Duration` value (0 for anything else).
fn duration_ms(v: Option<&Value<'_>>) -> u64 {
    match v {
        Some(Value::Record { fields, .. }) => match fields.first().map(|f| &f.value) {
            Some(Value::Int(ms, _)) => (*ms).max(0) as u64,
            _ => 0,
        },
        _ => 0,
    }
}

impl<'a> Interp<'a> {
    pub(crate) fn time_call(&mut self, name: &str, args: Vec<Value<'a>>, span: Span) -> Eval<'a> {
        if let Some(v) = self.frozen_call(name, &args) {
            return Ok(v);
        }
        // A generated test's clock (spec 11.5): sleeping moves it on.
        if self.faking("Clock") {
            let ms = match (name, args.first()) {
                ("sleep_ms", Some(Value::Int(ms, _))) => *ms,
                ("sleep", d) => duration_ms(d) as i128,
                _ => 0,
            };
            match name {
                "now_ms" | "sleep_ms" | "sleep" => {
                    let now = self.fake_clock(ms);
                    if name == "now_ms" {
                        return Ok(now.unwrap_or(Value::Unit));
                    }
                    return Ok(Value::Unit);
                }
                "monotonic_ms" => return Ok(self.fake_monotonic().unwrap_or(Value::Unit)),
                _ => {}
            }
        }
        Ok(match ("time", name) {
            // In a durable handler, the time the journal has (spec 8.5).
            ("time", "now_ms") => {
                let ms = self.journaled_time("time.now_ms", wall_ms(), span)?;
                Value::Int(ms, Prim::Int)
            }
            ("time", "zone_offset") => match (args.first(), args.get(1)) {
                (Some(Value::Str(zone)), Some(Value::Int(ms, _))) => {
                    let ms = (*ms).clamp(i64::MIN as i128, i64::MAX as i128) as i64;
                    match kavach_runtime_core::zones::offset_seconds(zone, ms) {
                        Some(s) => Value::Int(s as i128, Prim::Int),
                        None => Value::None,
                    }
                }
                _ => Value::None,
            },
            ("time", "monotonic_ms") => {
                static START: OnceLock<Instant> = OnceLock::new();
                Value::Int(
                    START.get_or_init(Instant::now).elapsed().as_millis() as i128,
                    Prim::Int,
                )
            }
            ("time", "sleep_ms") => {
                if let Some(Value::Int(ms, _)) = args.first() {
                    self.sleep_ms((*ms).max(0), span)?;
                }
                Value::Unit
            }
            ("time", "every_ms") => match args.first() {
                Some(Value::Int(ms, _)) => Value::Ticker((*ms).max(0) as u64),
                _ => Value::Ticker(0),
            },
            ("time", "after") => {
                let ms = duration_ms(args.first());
                let f = args.get(1).cloned().unwrap_or(Value::Unit);
                let work = crate::tasks::Work::After(ms, f, span);
                return self.spawn_work(work, span);
            }
            // A `Duration` is its milliseconds (spec 14.4).
            ("time", "sleep" | "every") => {
                let ms = duration_ms(args.first());
                if name == "every" {
                    Value::Ticker(ms)
                } else {
                    self.sleep_ms(ms as i128, span)?;
                    Value::Unit
                }
            }
            _ => return Err(self.panic(format!("std.time has no function `{name}`"), span)),
        })
    }
}

impl<'a> Interp<'a> {
    /// Waits `ms` milliseconds.
    fn sleep_ms(&mut self, ms: i128, span: Span) -> Result<(), crate::value::Flow<'a>> {
        let at = Instant::now() + std::time::Duration::from_millis(ms.max(0) as u64);
        self.journaled_sleep(at, span)
    }

    /// Waits until `at`. In a durable handler the wait's end goes into the
    /// journal, so after a restart only what is left is waited for, never
    /// more (spec 8.5, decision 0062).
    pub(crate) fn journaled_sleep(
        &mut self,
        at: Instant,
        span: Span,
    ) -> Result<(), crate::value::Flow<'a>> {
        let now = Instant::now();
        // Whole milliseconds, rounded up, so a wait never ends early.
        let ms = at
            .saturating_duration_since(now)
            .as_micros()
            .div_ceil(1_000) as i128;
        // A generated test's clock moves on at once (spec 11.5).
        if self.faking("Clock") {
            self.fake_clock(ms);
            return Ok(());
        }
        let wall = wall_ms();
        let due = self.journaled_time("time.sleep", wall + ms, span)?;
        let left = (due - wall).clamp(0, ms);
        let until = match left == ms {
            true => at,
            false => now + std::time::Duration::from_millis(left as u64),
        };
        self.sleep_until(until, span)
    }
}

impl<'a> Interp<'a> {
    /// `5.seconds` (spec 14.4): a `Duration`, when `base` is a whole number
    /// and `name` a unit.
    fn duration_of(&mut self, n: Value<'a>, factor: i64, span: Span) -> Eval<'a> {
        let factor = Value::Int(factor as i128, kavach_checker::Prim::Int);
        let ms = self.arith(kavach_ast::BinaryOp::Mul, n, factor, span)?;
        let Some(def) = self.c.resolution.std_type("Duration") else {
            return Err(self.panic("`Duration` is missing", span));
        };
        Ok(Value::Record {
            def,
            life: None,
            name: "Duration".into(),
            fields: vec![crate::value::Field {
                name: "ms".into(),
                value: ms,
                secret: false,
            }],
        })
    }

    pub(crate) fn duration(
        &mut self,
        base: &'a Expr,
        name: &kavach_ast::Ident,
        span: Span,
    ) -> Option<Eval<'a>> {
        let (_, factor) = kavach_checker::DURATION_UNITS
            .iter()
            .find(|(u, _)| *u == name.name)?;
        let ty = self.c.expr_types.get(&(base.span.start, base.span.end));
        let int = matches!(
            ty,
            Some(kavach_checker::Ty::Prim(kavach_checker::Prim::Int))
        ) || matches!(base.kind, kavach_ast::ExprKind::Int(_));
        if !int {
            return None;
        }
        Some(
            self.eval(base)
                .and_then(|n| self.duration_of(n, *factor, span)),
        )
    }
}

impl<'a> Interp<'a> {
    /// `time.freeze`, and the clock calls a frozen clock answers (spec
    /// 14.4). A second `freeze` moves only the wall clock, as a person
    /// changing the machine's clock would.
    fn frozen_call(&mut self, name: &str, args: &[Value<'a>]) -> Option<Value<'a>> {
        if let ("freeze", Some(Value::Int(ms, _))) = (name, args.first()) {
            let mono = self.frozen.as_ref().map_or(0, |c| c.get().1);
            match &self.frozen {
                Some(c) => c.set((*ms, mono)),
                None => self.frozen = Some(Rc::new(Cell::new((*ms, mono)))),
            }
            return Some(Value::Unit);
        }
        let clock = self.frozen.as_ref()?;
        let (now, mono) = clock.get();
        let slept = match (name, args.first()) {
            ("sleep_ms", Some(Value::Int(ms, _))) => (*ms).max(0),
            ("sleep", d) => duration_ms(d) as i128,
            ("now_ms", _) => return Some(Value::Int(now, Prim::Int)),
            ("monotonic_ms", _) => return Some(Value::Int(mono, Prim::Int)),
            _ => return None,
        };
        clock.set((now + slept, mono + slept));
        Some(Value::Unit)
    }
}
