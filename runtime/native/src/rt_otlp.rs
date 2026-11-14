//! OpenTelemetry export of `std.log` events in native code (spec 14.12):
//! the collector the compiler read from `[log.otlp]` (`OtlpConfigure`, at
//! the start of `main`), each event written (`OtlpExport`), and metrics
//! (`OtlpMetric`), sent by `kavach_runtime_net::otlp`; spans that end come
//! through the hook set here (`rt_scopes::SPAN_SINK`). A module and entry of its own, so only a
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
            let _ = crate::rt_scopes::SPAN_SINK.set(send_spans);
        }
        return 0;
    }
    if op == RtOp::OtlpMetric {
        let name = with_str(b, str::to_string);
        release(b);
        if let Some((endpoint, service)) = COLLECTOR.get() {
            let value = f64::from_bits(c as u64);
            kavach_runtime_net::otlp_metrics::record(endpoint, service, a, &name, value);
        }
        return 0;
    }
    let line = with_str(b, str::to_string);
    let name = with_str(c, str::to_string);
    release(b);
    release(c);
    if let Some((endpoint, service)) = COLLECTOR.get() {
        let text = line.split_once(' ').map_or(line.as_str(), |(_, t)| t);
        let span = crate::rt_scopes::current_span();
        otlp::export(endpoint, service, a, &name, text, span);
    }
    0
}

/// Spans that ended, to the collector.
fn send_spans(spans: Vec<kavach_runtime_core::trace::Finished>) {
    if let Some((endpoint, service)) = COLLECTOR.get() {
        otlp::export_spans(endpoint, service, spans);
    }
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_otlp, |op, a, b, c, code, line| {
    let _ = (code, line);
    otlp_op(op, a, b, c)
});
