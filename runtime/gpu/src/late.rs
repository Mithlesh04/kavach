//! The stop flags of the `map`s that didn't wait (decision 0061): a `map`
//! whose list stays on the GPU goes on at once, and whether one of its
//! items stopped is read later, at `finish`, oldest first.

use crate::Gpu;
use std::sync::Mutex;

/// What stopped an item: the code, its two operands, the line, and the
/// kernel (whose first lines give the messages of codes 100 and up).
pub type Stop = (u32, u32, u32, u32, String);

/// A stop flag not read yet, its kernel, and its work in the GPU's queue.
type Flag = (wgpu::Buffer, String, wgpu::SubmissionIndex);

/// What is known of the flags.
struct Late {
    /// Those not read yet, oldest first.
    flags: Vec<Flag>,
    /// The first stop already found in them.
    stop: Option<Stop>,
    /// How many have been read.
    read: u64,
}

static LATE: Mutex<Late> = Mutex::new(Late {
    flags: Vec::new(),
    stop: None,
    read: 0,
});

/// The most flags kept unread: past it, the oldest half is read (the GPU
/// has done them by then), so a long loop of maps keeps no more.
const MOST: usize = 64;

/// Keeps `staging`, the stop flag of a `map` of `kernel` that didn't wait.
pub(crate) fn keep(gpu: &Gpu, staging: wgpu::Buffer, kernel: &str, index: wgpu::SubmissionIndex) {
    let mut late = LATE.lock().unwrap_or_else(|e| e.into_inner());
    late.flags.push((staging, kernel.to_string(), index));
    if late.flags.len() > MOST {
        let old: Vec<_> = late.flags.drain(..MOST / 2).collect();
        late.read += old.len() as u64;
        let first = read(gpu, old);
        late.stop = late.stop.take().or(first);
    }
}

/// Waits for every `map` that didn't wait, and gives the first stop
/// among them, if any (spec 15.2).
pub fn finish() -> Option<Stop> {
    let gpu = crate::gpu()?;
    let mut late = LATE.lock().unwrap_or_else(|e| e.into_inner());
    let pending = std::mem::take(&mut late.flags);
    late.read += pending.len() as u64;
    let first = read(gpu, pending);
    late.stop.take().or(first)
}

/// How many flags have been kept so far, to pass to [`finish_before`].
pub fn flags_so_far() -> u64 {
    let late = LATE.lock().unwrap_or_else(|e| e.into_inner());
    late.read + late.flags.len() as u64
}

/// As [`finish`], for only the first `count` flags ever kept: what came
/// after them goes on.
pub fn finish_before(count: u64) -> Option<Stop> {
    let gpu = crate::gpu()?;
    let mut late = LATE.lock().unwrap_or_else(|e| e.into_inner());
    let due = count.saturating_sub(late.read).min(late.flags.len() as u64);
    let pending: Vec<_> = late.flags.drain(..due as usize).collect();
    late.read += due;
    let first = read(gpu, pending);
    late.stop.take().or(first)
}

/// Reads `flags` in order: the first that says an item stopped.
fn read(gpu: &Gpu, flags: Vec<Flag>) -> Option<Stop> {
    let mut first = None;
    for (staging, kernel, index) in flags {
        if gpu.wait_mapped(&staging, Some(index)).is_err() {
            continue;
        }
        let words: Vec<u32> = match staging.slice(..).get_mapped_range() {
            Ok(view) => view[..20]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
            Err(_) => continue,
        };
        staging.unmap();
        if first.is_none() && words[0] != 0 {
            first = Some((words[1], words[2], words[3], words[4], kernel));
        }
    }
    first
}
