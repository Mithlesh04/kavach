// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `map` on the device chosen for it (spec 15.2, 15.6), `read` of a list
//! left in GPU memory, and moving items to another device through main
//! memory. Work on a device that is lost meanwhile moves on to the next.

use crate::{Gpu, ITEMS};
use crate::{GpuError, Items, Kept, Resident, devices, gpu};
use std::sync::Arc;

/// Opens the GPU and compiles `wgsl`, so a later `map` with it starts
/// at once.
pub fn prepare(wgsl: &str) {
    if let Some(gpu) = gpu() {
        gpu.pipeline(wgsl, false);
    }
}

/// Runs `work` on the device chosen for kernel `wgsl` over `items`, with
/// the items where that device reads them: a list in another device's
/// memory comes over through main memory. When the device is lost
/// meanwhile, the work goes to the next one; `Unavailable` sends it to
/// the CPU.
pub(crate) fn on_device<T>(
    wgsl: &str,
    items: Items,
    mut work: impl FnMut(&'static Gpu, Items) -> Result<T, GpuError>,
) -> Result<T, GpuError> {
    let holder = match items {
        Items::Resident(r) => Some(r.device()),
        Items::Bytes(_) => None,
    };
    loop {
        let gpu = devices::pick(wgsl, holder).ok_or(GpuError::Unavailable)?;
        let done = match items {
            Items::Resident(r) if r.device() != gpu.index => {
                let mut bytes = vec![0u8; r.bytes()];
                read(r, &mut bytes)?;
                work(gpu, Items::Bytes(&bytes))
            }
            items => work(gpu, items),
        };
        match done {
            Err(GpuError::Unavailable) if gpu.is_lost() => continue,
            done => return done,
        }
    }
}

/// `items` mapped by kernel `wgsl` into `out`, one 32-bit word
/// (little-endian) per result, with `captures` (one word each) for the
/// values its lambda captured. An item may be several words (the fields
/// of a tuple or record), as many for each. Neither side is copied more
/// than the GPU needs; what stays in GPU memory comes back for later
/// `map`s. With `stay`, results that fit one dispatch stay on the GPU
/// only, not written into `out`: `read` brings them back when they are
/// needed. With `in_place` too (nothing else holds the items), results
/// as many words as the items are written over them, in their own
/// buffer.
pub fn map(
    wgsl: &str,
    items: Items,
    captures: &[u32],
    out: &mut [u8],
    stay: bool,
    in_place: bool,
) -> Result<Kept, GpuError> {
    on_device(wgsl, items, |gpu, items| {
        map_on(gpu, wgsl, items, captures, out, stay, in_place)
    })
}

/// `map` on device `gpu`, with the items in its memory or in main memory.
fn map_on(
    gpu: &'static Gpu,
    wgsl: &str,
    items: Items,
    captures: &[u32],
    out: &mut [u8],
    stay: bool,
    in_place: bool,
) -> Result<Kept, GpuError> {
    // A record or tuple result is several words per item: the kernel's
    // header says how many (`// out 4`).
    let out_width = out_width(wgsl);
    let words = out.len() / 4;
    let n = words / out_width;
    let bytes = match items {
        Items::Resident(r) => {
            // Each item's fields are all read before its results are
            // written, at the same words.
            let over = stay && in_place && r.words == words && wgsl.contains(ITEMS);
            let pipeline = gpu.pipeline(wgsl, over);
            let results = gpu.run(
                &pipeline,
                &r.buffer,
                n,
                words,
                captures,
                (!stay).then_some(out),
                stay.then_some(wgsl),
                over.then_some(&*r.buffer),
            )?;
            let buffer = match over {
                true => r.buffer.clone(),
                false => Arc::new(results),
            };
            return Ok(Kept {
                items: None,
                results: Some(Resident { buffer, words, gpu }),
            });
        }
        Items::Bytes(b) => b,
    };
    let pipeline = gpu.pipeline(wgsl, false);
    let width = (bytes.len() / 4).checked_div(n).unwrap_or(1).max(1);
    let chunk = (gpu.max_items / width.max(out_width)).max(1);
    if n > chunk {
        let items = bytes.chunks(4 * width * chunk);
        for (items, out) in items.zip(out.chunks_mut(4 * out_width * chunk)) {
            let input = gpu.load(items);
            let words = out.len() / 4;
            gpu.run(
                &pipeline,
                &input,
                words / out_width,
                words,
                captures,
                Some(out),
                None,
                None,
            )?;
        }
        return Ok(Kept {
            items: None,
            results: None,
        });
    }
    let input = Arc::new(gpu.load(bytes));
    let results = gpu.run(
        &pipeline,
        &input,
        n,
        words,
        captures,
        (!stay).then_some(out),
        stay.then_some(wgsl),
        None,
    )?;
    let resident = |buffer, words| Some(Resident { buffer, words, gpu });
    Ok(Kept {
        items: resident(input, n * width),
        results: resident(Arc::new(results), words),
    })
}

/// How many words each item's result takes: 1, or what a kernel's header
/// says for a record or tuple result (`// out 4`).
pub fn out_width(wgsl: &str) -> usize {
    (wgsl.lines().take_while(|l| l.starts_with("//")))
        .find_map(|l| l.strip_prefix("// out "))
        .and_then(|w| w.parse().ok())
        .unwrap_or(1)
}

/// The words of `items`, in GPU memory, copied into `out`.
pub fn read(items: &Resident, out: &mut [u8]) -> Result<(), GpuError> {
    let gpu = items.gpu;
    let size = (items.bytes().max(4)) as u64;
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(&items.buffer, 0, &staging, 0, size);
    gpu.queue.submit([encoder.finish()]);
    gpu.wait_mapped(&staging, None)?;
    let view = staging
        .slice(..)
        .get_mapped_range()
        .map_err(|_| GpuError::Unavailable)?;
    let n = out.len().min(items.bytes());
    out[..n].copy_from_slice(&view[..n]);
    drop(view);
    staging.unmap();
    Ok(())
}
