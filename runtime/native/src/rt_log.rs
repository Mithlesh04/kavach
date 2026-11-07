//! `std.log`'s lines in native code (spec 14.12), written as the `[log]`
//! settings in `kavach.toml` say. The compiler read them and passes them
//! once, at the start of `main` (`LogConfigure`); the writing itself is
//! `kavach_runtime_core::log`, the interpreter's too.

use crate::heap::release;
use crate::shapes::with_str;
use kavach_kir::RtOp;
use kavach_runtime_core::json::{self, Tree};
use kavach_runtime_core::log::Settings;
use std::sync::OnceLock;

static SETTINGS: OnceLock<Settings> = OnceLock::new();

/// The log ops (operands owned): the settings; or a line `b` at level `a`,
/// written when `a` is at least `c`, the event's lowest level, which the
/// compiler worked out.
pub fn log_op(op: RtOp, a: i64, b: i64, c: i64) -> i64 {
    match op {
        RtOp::LogConfigure => {
            let settings = with_str(a, settings_of);
            release(a);
            let _ = SETTINGS.set(settings);
        }
        _ => {
            let line = with_str(b, str::to_string);
            release(b);
            let settings = SETTINGS.get_or_init(Settings::default);
            if a >= c {
                crate::rt_tasks::turn();
                settings.to_file(&line);
                if settings.console {
                    let shown = kavach_runtime_core::log::for_console(&line);
                    crate::rt::out(&format!("{shown}\n"));
                }
            }
        }
    }
    0
}

/// The settings from the description the compiler wrote
/// (`compiler/kir/src/lower_std.rs`).
fn settings_of(text: &str) -> Settings {
    let d = Settings::default();
    let Ok(Tree::Object(fields)) = json::parse(text, &json::Limits::default()) else {
        return d;
    };
    let field = |name: &str| fields.iter().find(|(k, _)| k == name).map(|(_, v)| v);
    let number = |name: &str| match field(name) {
        Some(Tree::Int(n)) => Some(*n),
        _ => None,
    };
    Settings {
        level: number("level").unwrap_or(d.level),
        console: !matches!(field("console"), Some(Tree::Bool(false))),
        file: match field("file") {
            Some(Tree::Text(f)) => Some(f.into()),
            _ => None,
        },
        max_bytes: number("max_bytes").map_or(d.max_bytes, |n| n.max(0) as u64),
        keep: number("keep").map_or(d.keep, |n| n.clamp(0, 1000) as u32),
        // Each call already carries its event's lowest level, and export
        // has its own ops (`rt_otlp.rs`).
        levels: d.levels,
        otlp: None,
    }
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_log, |op, a, b, c, code, line| {
    let _ = (code, line);
    log_op(op, a, b, c)
});
