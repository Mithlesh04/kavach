// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Reductions: `sum`, `min` and `max` on the GPU. [`reduce_later`] puts one
//! in the GPU's queue and goes on, so a loop's next chunk is read and
//! copied over while the GPU works (spec 15.2, decision 0067); its
//! [`Later::wait`] gives the partials when they are needed.

use crate::{Gpu, GpuError, Items, Pending, dispatch};

/// A reduction in the GPU's queue.
pub struct Later {
    staging: wgpu::Buffer,
    index: wgpu::SubmissionIndex,
    words: usize,
    /// The device it runs on, which counts it as work until it is read.
    gpu: &'static Gpu,
    _pending: Pending,
}

/// Queues reduce kernel `wgsl` (`compiler/backend/wgsl/src/reduce.rs`) over
/// the `n` items in `items`. Only its partials will come back.
pub fn reduce_later(
    wgsl: &str,
    items: Items,
    n: usize,
    captures: &[u32],
) -> Result<Later, GpuError> {
    crate::map::on_device(wgsl, items, |gpu, items| {
        later_on(gpu, wgsl, items, n, captures)
    })
}

/// `reduce_later` on device `gpu`.
fn later_on(
    gpu: &'static Gpu,
    wgsl: &str,
    items: Items,
    n: usize,
    captures: &[u32],
) -> Result<Later, GpuError> {
    if n > gpu.max_items {
        return Err(GpuError::Unavailable);
    }
    let pipeline = gpu.pipeline(wgsl, false);
    let (x, y) = dispatch(n);
    let words = 3 * (x * y) as usize;
    let run = match items {
        Items::Resident(r) => gpu.submit(&pipeline, &r.buffer, n, words, captures, true, None),
        Items::Bytes(b) => {
            let input = gpu.load(b);
            gpu.submit(&pipeline, &input, n, words, captures, true, None)
        }
    };
    Ok(Later {
        staging: run.staging,
        index: run.index,
        words,
        gpu,
        _pending: run.pending,
    })
}

impl Later {
    /// Waits for the GPU to reach it, and gives its partials: three words
    /// for each workgroup, in order, for the caller to combine. Work queued
    /// after it goes on.
    pub fn wait(self) -> Result<Vec<u32>, GpuError> {
        let gpu = self.gpu;
        let mut out = vec![0u8; self.words * 4];
        let back = out.len() as u64;
        gpu.read_back(&self.staging, back, Some(&mut out), Some(self.index))?;
        Ok(out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect())
    }
}

/// The partials of reduce kernel `wgsl` over the `n` items in `items`,
/// waiting for them.
pub fn reduce(wgsl: &str, items: Items, n: usize, captures: &[u32]) -> Result<Vec<u32>, GpuError> {
    reduce_later(wgsl, items, n, captures)?.wait()
}
