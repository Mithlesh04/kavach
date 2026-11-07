//! Buffers of items and results no list holds any more, kept for the next
//! `map` of the same size. A new buffer is filled with zeros before its first use;
//! one used before isn't, so a frame loop writes into the buffers of the
//! frames before it (300 frames of 1 million particles in CUDA: 75 ms
//! with a new buffer each frame, 44 ms with two in turn).

use std::sync::Mutex;

/// The usage of every buffer of items or results.
pub(crate) const RESULTS: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE
    .union(wgpu::BufferUsages::COPY_SRC)
    .union(wgpu::BufferUsages::COPY_DST);

/// The spare buffers, the newest last.
static SPARE: Mutex<Vec<wgpu::Buffer>> = Mutex::new(Vec::new());

/// The most spare buffers kept: past it, the oldest is let go of.
const MOST: usize = 4;

/// Keeps `buffer` for later, when it is one of those.
pub(crate) fn give(buffer: &wgpu::Buffer) {
    if buffer.usage() != RESULTS {
        return;
    }
    let mut spare = SPARE.lock().unwrap_or_else(|e| e.into_inner());
    spare.push(buffer.clone());
    if spare.len() > MOST {
        spare.remove(0);
    }
}

/// A spare result buffer of `size` bytes, if one is kept.
pub(crate) fn take(size: u64) -> Option<wgpu::Buffer> {
    let mut spare = SPARE.lock().unwrap_or_else(|e| e.into_inner());
    let at = spare.iter().rposition(|b| b.size() == size)?;
    Some(spare.remove(at))
}
