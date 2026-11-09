//! Items copied to the GPU through buffers of our own, used in turn.
//!
//! `queue.write_buffer` copies into a staging buffer of `wgpu`'s, and makes a
//! new one when its earlier ones are still in use: making a 64 MB one took
//! 100–280 ms, every few chunks of a stream (benchmarks/gpu). These buffers
//! are made once and written again once the GPU has copied out of them.

use super::Gpu;
use std::sync::Mutex;

/// A buffer the CPU writes items into, and the copy that last read it.
struct Upload {
    buffer: wgpu::Buffer,
    used: wgpu::SubmissionIndex,
}

/// The upload buffers, the most recently used last.
static RING: Mutex<Vec<Upload>> = Mutex::new(Vec::new());

/// Buffers of one size used in turn: while the GPU copies out of one, the
/// next chunk's items are written into the other.
const TURNS: usize = 2;

/// The most upload buffers kept, of any sizes: past it, the oldest is let go.
const MOST: usize = 4;

/// Writes smaller than this go through `queue.write_buffer`, whose staging
/// buffers of that size are quick to make.
const LEAST: usize = 1 << 20;

impl Gpu {
    /// Copies `items` into the start of `buffer`, before the next work.
    pub(crate) fn upload(&self, buffer: &wgpu::Buffer, items: &[u8]) {
        let size = items.len() as u64;
        if items.len() < LEAST || !size.is_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT) {
            self.queue.write_buffer(buffer, 0, items);
            return;
        }
        let mut ring = RING.lock().unwrap_or_else(|e| e.into_inner());
        let same = ring.iter().filter(|u| u.buffer.size() == size).count();
        let staging = match ring.iter().position(|u| u.buffer.size() == size) {
            Some(at) if same >= TURNS => {
                let old = ring.remove(at);
                let written =
                    self.wait_mapped_for(&old.buffer, wgpu::MapMode::Write, Some(old.used));
                if written.is_err() {
                    self.queue.write_buffer(buffer, 0, items);
                    return;
                }
                old.buffer
            }
            _ => self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("kavach upload"),
                size,
                usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            }),
        };
        match staging.slice(..).get_mapped_range_mut() {
            Ok(mut view) => view.copy_from_slice(items),
            Err(_) => {
                self.queue.write_buffer(buffer, 0, items);
                return;
            }
        }
        staging.unmap();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&staging, 0, buffer, 0, size);
        let used = self.queue.submit([encoder.finish()]);
        ring.push(Upload {
            buffer: staging,
            used,
        });
        if ring.len() > MOST {
            ring.remove(0);
        }
    }
}
