// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Buffers of items and results no list holds any more, kept for the next
//! `map` of the same size. A new buffer is filled with zeros before its first use;
//! one used before isn't, so a frame loop writes into the buffers of the
//! frames before it (300 frames of 1 million particles in CUDA: 75 ms
//! with a new buffer each frame, 44 ms with two in turn).

use std::sync::Mutex;

/// The usage of every buffer of items or results: also as vertices or
/// instances, so a draw reads a `map`'s results where they are (15.3).
pub(crate) const RESULTS: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE
    .union(wgpu::BufferUsages::COPY_SRC)
    .union(wgpu::BufferUsages::COPY_DST)
    .union(wgpu::BufferUsages::VERTEX);

/// The spare buffers, each with its device's number, the newest last.
static SPARE: Mutex<Vec<(usize, wgpu::Buffer)>> = Mutex::new(Vec::new());

/// The most spare buffers kept: past it, the oldest is let go of.
const MOST: usize = 4;

/// Keeps `buffer`, of device `device`, for later, when it is one of those.
pub(crate) fn give(device: usize, buffer: &wgpu::Buffer) {
    if buffer.usage() != RESULTS {
        return;
    }
    let mut spare = SPARE.lock().unwrap_or_else(|e| e.into_inner());
    spare.push((device, buffer.clone()));
    if spare.len() > MOST {
        spare.remove(0);
    }
}

/// A spare result buffer of device `device`, of `size` bytes, if one is
/// kept.
pub(crate) fn take(device: usize, size: u64) -> Option<wgpu::Buffer> {
    let mut spare = SPARE.lock().unwrap_or_else(|e| e.into_inner());
    let at = (spare.iter()).rposition(|(d, b)| *d == device && b.size() == size)?;
    Some(spare.remove(at).1)
}
