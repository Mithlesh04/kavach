// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Spans and the GPU's work shown live in the Tracy profiler (spec 14.12,
//! decision 0123), through `tracy-client`'s safe calls. `start` turns the
//! client on and gives `kavach_runtime_core::profiler` its hooks; only
//! `kavach run --tracy` and a program built with `--tracy` call it.

use kavach_runtime_core::profiler::{self, Profiler};
use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};
use tracy_client::{Client, GpuContext, GpuContextType, Span};

thread_local! {
    /// This thread's open zones, innermost last: each span's id, its zone,
    /// and whether the span has ended. A zone ends when it is on top, as
    /// Tracy's zones nest.
    static ZONES: RefCell<Vec<(u64, Span, bool)>> = const { RefCell::new(Vec::new()) };
}

/// The GPU's context, made with its clock at the first timed work.
fn gpu_context() -> &'static Mutex<Option<GpuContext>> {
    static GPU: OnceLock<Mutex<Option<GpuContext>>> = OnceLock::new();
    GPU.get_or_init(Mutex::default)
}

/// Starts Tracy's client, which Tracy's viewer on this machine connects
/// to, and sends it every span from now on.
pub fn start() {
    let _ = Client::start();
    profiler::set(Profiler { begin, end, gpu });
}

fn begin(id: u64, name: &str) {
    let Some(client) = Client::running() else {
        return;
    };
    let zone = client.span_alloc(Some(name), name, "kavach", 0, 0);
    ZONES.with(|z| z.borrow_mut().push((id, zone, false)));
}

fn end(id: u64) {
    ZONES.with(|z| close(&mut z.borrow_mut(), id));
}

/// Marks span `id` ended in `zones`, then ends every ended zone on top,
/// innermost first, by letting it go.
fn close<Z>(zones: &mut Vec<(u64, Z, bool)>, id: u64) {
    if let Some(open) = zones.iter_mut().rev().find(|(i, _, _)| *i == id) {
        open.2 = true;
    }
    while zones.last().is_some_and(|(_, _, ended)| *ended) {
        zones.pop();
    }
}

fn gpu(name: &str, start: u64, end: u64, period: f32) {
    let Some(client) = Client::running() else {
        return;
    };
    let mut context = gpu_context().lock().unwrap_or_else(|e| e.into_inner());
    if context.is_none() {
        *context =
            (client.new_gpu_context(Some("GPU"), GpuContextType::Invalid, start as i64, period))
                .ok();
    }
    let Some(context) = context.as_ref() else {
        return;
    };
    if let Ok(mut span) = context.span_alloc(name, name, "kavach", 0) {
        span.end_zone();
        span.upload_timestamp_start(start as i64);
        span.upload_timestamp_end(end as i64);
    }
}

#[cfg(test)]
mod tests {
    /// Zones end innermost first, and one that ends under another waits
    /// for it: Tracy's zones nest.
    #[test]
    fn zones_end_only_from_the_top() {
        let mut zones = vec![(1, "outer", false), (2, "inner", false)];
        super::close(&mut zones, 1);
        assert_eq!(zones.len(), 2, "the outer zone waits for the inner one");
        super::close(&mut zones, 2);
        assert!(zones.is_empty());
        let mut zones = vec![(1, "outer", false), (2, "inner", false)];
        super::close(&mut zones, 2);
        assert_eq!(zones, vec![(1, "outer", false)]);
    }
}
