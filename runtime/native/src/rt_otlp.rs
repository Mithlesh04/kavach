//! OpenTelemetry export of `std.log` events in native code (spec 14.12):
//! the collector the compiler read from `[log.otlp]` (`OtlpConfigure`, at
//! the start of `main`), and each event written (`OtlpExport`), sent by
//! `kavach_runtime_net::otlp`. A module and entry of its own, so only a
//! program that exports links the network's code (decision 0024).

use crate::heap::release;
use crate::shapes::with_str;
use kavach_kir::RtOp;
use kavach_runtime_net::otlp;
use std::sync::OnceLock;

/// The collector's endpoint and the service name.
static COLLECTOR: OnceLock<(String, String)> = OnceLock::new();

/// The OTLP ops (operands owned): `endpoint` and `service` (as a line
/// each, in one text); or an event at level `a`, its line `b` (the level
/// and the printed form) and its type's name `c`.
pub fn otlp_op(op: RtOp, a: i64, b: i64, c: i64) -> i64 {
    if op == RtOp::OtlpConfigure {
        let settings = with_str(a, str::to_string);
        release(a);
        let (endpoint, service) = settings.split_once('\n').unwrap_or((&settings, ""));
        if COLLECTOR
            .set((endpoint.to_string(), service.to_string()))
            .is_ok()
        {
            // The last events go before the program ends.
            crate::rt::at_exit(otlp::finish);
        }
        return 0;
    }
    let line = with_str(b, str::to_string);
    let name = with_str(c, str::to_string);
    release(b);
    release(c);
    if let Some((endpoint, service)) = COLLECTOR.get() {
        let text = line.split_once(' ').map_or(line.as_str(), |(_, t)| t);
        otlp::export(endpoint, service, a, &name, text);
    }
    0
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_otlp, |op, a, b, c, code, line| {
    let _ = (code, line);
    otlp_op(op, a, b, c)
});
