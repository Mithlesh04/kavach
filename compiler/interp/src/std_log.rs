//! `std.log` in the interpreter (spec 14.12): events written where
//! `[log]` in `kavach.toml` says, spans run as blocks given to a call
//! (4.8), seeded random numbers (14.15), and metrics, all sent to the
//! collector of `[log.otlp]` by `kavach_runtime_net` (decision 0107).

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{Block, Expr, ExprKind};
use kavach_runtime_core::fs_scope::{Scope, current_span, leave, span};
use kavach_runtime_core::log::Settings as LogSettings;
use kavach_runtime_core::trace::{parse_traceparent, traceparent};
use std::sync::{Arc, Mutex};

impl<'a> Interp<'a> {
    /// One line from `std.log` (spec 14.12): to the console and the file
    /// `[log]` in `kavach.toml` names, unless below the level for the
    /// event's type and module.
    pub(crate) fn log_write(&mut self, level: i64, line: &str, event: &Value<'_>) {
        let def = match event {
            Value::Record { def, .. } => Some(*def),
            Value::Variant { union, .. } => Some(*union),
            _ => None,
        };
        let res = &self.c.resolution;
        let (module, name) = def.map_or((String::new(), String::new()), |d| {
            let start = res.defs[d].span.map_or(0, |s| s.start);
            let module = res.modules[res.module_at(start)].name.clone();
            (module, res.defs[d].name.clone())
        });
        let span = current_span(&self.fs_scopes);
        let settings = self.log_settings();
        if level < settings.level_for(&module, &name) {
            return;
        }
        if let Some((endpoint, service)) = &settings.otlp {
            let text = line.split_once(' ').map_or(line, |(_, t)| t);
            kavach_runtime_net::otlp::export(endpoint, service, level, &name, text, span);
        }
        settings.to_file(line);
        if settings.console {
            self.out
                .push_str(&kavach_runtime_core::log::for_console(line));
            self.out.push('\n');
            self.show_live();
        }
    }

    /// The `[log]` settings, read from `kavach.toml` once.
    fn log_settings(&mut self) -> &mut LogSettings {
        self.log.get_or_insert_with(|| {
            kavach_driver::Config::for_file(std::path::Path::new(&self.c.path))
                .unwrap_or_default()
                .log_settings()
        })
    }

    /// `log.span(name, from: header) { … }` and `random.seeded(seed) { … }`
    /// (spec 4.8): the block, inside what the call sets up, which ends
    /// however the block ends.
    pub(crate) fn scoped(&mut self, call: &'a Expr, body: &'a Block) -> Eval<'a> {
        let ExprKind::Call { callee, args, .. } = &call.kind else {
            return self.block(body);
        };
        let seeded = matches!(&callee.kind, ExprKind::Field { name, .. } if name.name == "seeded");
        let mut given = Vec::new();
        for a in args {
            given.push((
                a.name.as_ref().map(|n| n.name.as_str()),
                self.eval(&a.value)?,
            ));
        }
        let text = |at: usize, named: &str| {
            given
                .iter()
                .enumerate()
                .find(|(i, (n, _))| *n == Some(named) || (n.is_none() && *i == at))
                .and_then(|(_, (_, v))| match v {
                    Value::Str(s) => Some(s.to_string()),
                    _ => None,
                })
        };
        let depth = self.fs_scopes.len();
        if seeded {
            let seed = match given.first() {
                Some((_, Value::Int(n, _))) => *n as u64,
                _ => 0,
            };
            let g = kavach_runtime_core::seeded::Seeded::new(seed);
            self.fs_scopes.push(Scope::Seeded(Arc::new(Mutex::new(g))));
        } else {
            let name = text(0, "name").unwrap_or_default();
            let remote = text(1, "from").and_then(|h| parse_traceparent(&h));
            self.fs_scopes.push(span(&self.fs_scopes, &name, remote));
        }
        let result = self.block(body);
        // A panic sends nothing: the panic says what happened.
        if matches!(result, Err(Flow::Panic(..))) {
            self.fs_scopes.truncate(depth);
            return result;
        }
        let failed = matches!(result, Err(Flow::Fail(_))).then_some("failed");
        let ended = leave(&mut self.fs_scopes, depth, failed);
        if !ended.is_empty()
            && let Some((endpoint, service)) = self.log_settings().otlp.clone()
        {
            kavach_runtime_net::otlp::export_spans(&endpoint, &service, ended);
        }
        result
    }

    /// `log.metric_id`, `trace_id_text` and `traceparent_text`.
    pub(crate) fn log_call(&mut self, name: &str, args: &[Value<'a>]) -> Eval<'a> {
        let span = current_span(&self.fs_scopes);
        Ok(match name {
            "metric_id" => {
                let kind = match args.first() {
                    Some(Value::Int(n, _)) => *n as i64,
                    _ => 0,
                };
                let value = match args.get(2) {
                    Some(Value::Float(f)) => *f,
                    _ => 0.0,
                };
                if let (Some(Value::Str(metric)), Some((endpoint, service))) =
                    (args.get(1), self.log_settings().otlp.clone())
                {
                    kavach_runtime_net::otlp_metrics::record(
                        &endpoint, &service, kind, metric, value,
                    );
                }
                Value::Unit
            }
            "name_span_id" => {
                if let Some(Value::Str(name)) = args.first() {
                    kavach_runtime_core::fs_scope::rename_span(&mut self.fs_scopes, name);
                }
                Value::Unit
            }
            "trace_id_text" => {
                self.str_value(&span.map_or(String::new(), |s| format!("{:032x}", s.trace)))
            }
            _ => self.str_value(&span.map_or(String::new(), traceparent)),
        })
    }
}
