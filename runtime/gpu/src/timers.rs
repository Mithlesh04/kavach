// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The GPU's own time for its work (spec 14.12): with `KAVACH_TRACE` on
//! and a GPU that has timestamp queries, a frame's drawing and each GPU
//! `map` write the GPU's clock before and after the work, and the time
//! between goes into the Chrome trace on its own row. Read back without
//! waiting: each submission looks for timers whose results are in.

use kavach_runtime_core::chrome_trace;
use std::sync::{Arc, Mutex, OnceLock};

/// The features timing needs, when the GPU has them and a trace is on.
pub(crate) fn features(adapter: &wgpu::Adapter) -> wgpu::Features {
    let wanted = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    let traced = chrome_trace::on() || kavach_runtime_core::profiler::on();
    if traced && adapter.features().contains(wanted) {
        wanted
    } else {
        wgpu::Features::empty()
    }
}

/// One timed piece of work: its two timestamps, where they are resolved,
/// and when the CPU sent it.
pub(crate) struct Timer {
    name: &'static str,
    set: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    read: Arc<wgpu::Buffer>,
    sent: u128,
}

/// A timer sent, and whether its results were read back (once known).
type Waiting = (Timer, Arc<Mutex<Option<bool>>>);

/// Timers sent, waiting for the GPU's results.
fn waiting() -> std::sync::MutexGuard<'static, Vec<Waiting>> {
    static WAITING: OnceLock<Mutex<Vec<Waiting>>> = OnceLock::new();
    WAITING
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Starts timing work `name` recorded into `encoder`, when timing is on.
pub(crate) fn start(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    name: &'static str,
) -> Option<Timer> {
    if !device
        .features()
        .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
    {
        return None;
    }
    let set = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("kavach timer"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let buffer = |usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kavach timer"),
            size: 16,
            usage,
            mapped_at_creation: false,
        })
    };
    let resolved = buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC);
    let read = Arc::new(buffer(
        wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    ));
    encoder.write_timestamp(&set, 0);
    Some(Timer {
        name,
        set,
        resolved,
        read,
        sent: 0,
    })
}

/// Ends `timer` in `encoder`, before the encoder is finished.
pub(crate) fn stop(encoder: &mut wgpu::CommandEncoder, timer: &Timer) {
    encoder.write_timestamp(&timer.set, 1);
    encoder.resolve_query_set(&timer.set, 0..2, &timer.resolved, 0);
    encoder.copy_buffer_to_buffer(&timer.resolved, 0, &timer.read, 0, 16);
}

/// After the work's submission: asks for `timer`'s results, and writes
/// those of earlier timers that are in.
pub(crate) fn sent(device: &wgpu::Device, queue: &wgpu::Queue, timer: Option<Timer>) {
    if let Some(mut timer) = timer {
        timer.sent = kavach_runtime_core::trace::now_nanos();
        let ready = Arc::new(Mutex::new(None));
        let done = ready.clone();
        timer
            .read
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                *done.lock().unwrap_or_else(|e| e.into_inner()) = Some(r.is_ok());
            });
        waiting().push((timer, ready));
    }
    let _ = device.poll(wgpu::PollType::Poll);
    let period = f64::from(queue.get_timestamp_period());
    waiting().retain(|(timer, ready)| {
        let state = *ready.lock().unwrap_or_else(|e| e.into_inner());
        match state {
            None => true,
            Some(false) => false,
            Some(true) => {
                let (start, end) = match timer.read.slice(..).get_mapped_range() {
                    Ok(bytes) => {
                        let at = |i: usize| {
                            u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap_or([0; 8]))
                        };
                        (at(0), at(1))
                    }
                    Err(_) => return false,
                };
                timer.read.unmap();
                kavach_runtime_core::profiler::gpu(timer.name, start, end, period as f32);
                let nanos = (end.saturating_sub(start) as f64 * period) as u128;
                chrome_trace::write_gpu(timer.name, timer.sent, nanos);
                false
            }
        }
    });
}
