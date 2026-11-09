//! GPU compute for `gpu fn` (spec 15, decision 0044), through `wgpu`.
//!
//! A kernel is WGSL from `compiler/backend/wgsl`, with four buffers of
//! 32-bit words: the items, the results, `info` (the item count, then the
//! captured values) and `stop` (set by the first item that panics: a
//! flag, what happened, and its two operands).

use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;

mod device;
use device::{Gpu, gpu};
pub use device::{available, ready};

/// Items per workgroup: the size every kernel declares.
pub const WORKGROUP: u32 = 256;

/// Why the GPU gave no list.
#[derive(Debug, PartialEq)]
pub enum GpuError {
    /// No usable GPU, or it failed: the CPU does the work instead.
    Unavailable,
    /// An item stopped as the CPU would panic: what happened (the codes
    /// of `compiler/backend/wgsl`) and its two operands.
    Stopped {
        code: u32,
        a: u32,
        b: u32,
        line: u32,
    },
}

/// A list's items in GPU memory, which a later `map` reads without
/// copying them over again.
#[derive(Clone)]
pub struct Resident {
    buffer: Arc<wgpu::Buffer>,
    /// Its 32-bit words: an item may take several.
    words: usize,
}

/// The last holder of a result buffer leaves it for a later `map`.
impl Drop for Resident {
    fn drop(&mut self) {
        if Arc::strong_count(&self.buffer) == 1 {
            spare::give(&self.buffer);
        }
    }
}

impl Resident {
    /// The bytes it takes in GPU memory.
    pub fn bytes(&self) -> usize {
        4 * self.words
    }
}

/// The items a `map` reads: 32-bit words (little-endian) to copy over, or
/// ones already in GPU memory.
pub enum Items<'a> {
    Bytes(&'a [u8]),
    Resident(&'a Resident),
}

/// What a `map` leaves in GPU memory, for later ones: the items it copied
/// over, and its results. Each is `None` when the work took more than one
/// dispatch, or the items were resident already.
pub struct Kept {
    pub items: Option<Resident>,
    pub results: Option<Resident>,
}

/// Buffer `buffer` bound at `binding`.
fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

/// How every kernel declares its items (`compiler/backend/wgsl`), and
/// how one that writes its results over them does.
const ITEMS: &str = "var<storage, read> items:";
const ITEMS_WRITTEN: &str = "var<storage, read_write> items:";

impl Gpu {
    /// A buffer holding `contents`.
    fn init(&self, contents: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage,
            })
    }

    /// A buffer holding a list's items, to read or write over. A new buffer
    /// made with its contents has them written twice, first as zeros
    /// (`wgpu`, mapped at creation), so it is made empty and written once,
    /// when no spare one is there.
    fn load(&self, items: &[u8]) -> wgpu::Buffer {
        let size = items.len().max(4) as u64;
        let buffer = spare::take(size).unwrap_or_else(|| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size,
                usage: spare::RESULTS,
                mapped_at_creation: false,
            })
        });
        self.upload(&buffer, items);
        buffer
    }

    /// Maps `staging` to read, waiting for submission `index`, or for all
    /// the GPU's work so far.
    fn wait_mapped(
        &self,
        staging: &wgpu::Buffer,
        index: Option<wgpu::SubmissionIndex>,
    ) -> Result<(), GpuError> {
        self.wait_mapped_for(staging, wgpu::MapMode::Read, index)
    }

    /// Maps `staging` to read or write (`mode`), as `wait_mapped`.
    fn wait_mapped_for(
        &self,
        staging: &wgpu::Buffer,
        mode: wgpu::MapMode,
        index: Option<wgpu::SubmissionIndex>,
    ) -> Result<(), GpuError> {
        let ready = Arc::new(Mutex::new(None));
        let done = ready.clone();
        staging.slice(..).map_async(mode, move |r| {
            *done.lock().unwrap_or_else(|e| e.into_inner()) = Some(r.is_ok());
        });
        let wait = wgpu::PollType::Wait {
            submission_index: index,
            timeout: None,
        };
        self.device.poll(wait).map_err(|_| GpuError::Unavailable)?;
        match *ready.lock().unwrap_or_else(|e| e.into_inner()) {
            Some(true) => Ok(()),
            _ => Err(GpuError::Unavailable),
        }
    }

    /// One dispatch over the `n` items in buffer `input`, which fit the
    /// buffers, writing as many words of results into `out`, or leaving
    /// them on the GPU when there is no `out`. Gives the buffer the
    /// results are in, on the GPU. It waits for the work, so a panic is
    /// reported before the program goes on; but with no `out` and a
    /// `late` kernel, it goes on at once, and `late::finish` reads the
    /// stop flag later (decision 0061). With `into`, the results go
    /// there: the items' own buffer, for a `map` that writes over them.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        pipeline: &wgpu::ComputePipeline,
        input: &wgpu::Buffer,
        n: usize,
        out_words: usize,
        captures: &[u32],
        out: Option<&mut [u8]>,
        late: Option<&str>,
        into: Option<&wgpu::Buffer>,
    ) -> Result<wgpu::Buffer, GpuError> {
        let run = self.submit(pipeline, input, n, out_words, captures, out.is_some(), into);
        if let (None, Some(kernel)) = (&out, late) {
            late::keep(self, run.staging, kernel, run.index);
            return Ok(run.output);
        }
        self.read_back(&run.staging, run.back, out, None)?;
        Ok(run.output)
    }

    /// Queues one dispatch, and the copies of its stop flag, and of its
    /// `out_words` words of results when `back`, for reading later.
    #[allow(clippy::too_many_arguments)]
    fn submit(
        &self,
        pipeline: &wgpu::ComputePipeline,
        input: &wgpu::Buffer,
        n: usize,
        out_words: usize,
        captures: &[u32],
        back: bool,
        into: Option<&wgpu::Buffer>,
    ) -> Submitted {
        let bytes =
            |words: &[u32]| -> Vec<u8> { words.iter().flat_map(|w| w.to_le_bytes()).collect() };
        let init = |contents: &[u8], usage| self.init(contents, usage);
        let mut info = vec![n as u32];
        info.extend_from_slice(captures);
        let info = init(&bytes(&info), wgpu::BufferUsages::STORAGE);
        let out_size = (out_words.max(1) * 4) as u64;
        let output = into.cloned().or_else(|| spare::take(out_size));
        let output = output.unwrap_or_else(|| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: out_size,
                usage: spare::RESULTS,
                mapped_at_creation: false,
            })
        });
        let stop = init(
            &[0u8; 20],
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        // Only the stop words come back when the results stay.
        let back = if back { out_size } else { 0 };
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: back + 20,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: if into.is_some() {
                &self.in_place
            } else {
                &self.layout
            },
            entries: &[
                entry(0, input),
                entry(1, &output),
                entry(2, &info),
                entry(3, &stop),
            ],
        });
        let (x, y) = dispatch(n);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(x, y, 1);
        }
        if back > 0 {
            encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, out_size);
        }
        encoder.copy_buffer_to_buffer(&stop, 0, &staging, back, 20);
        let index = self.queue.submit([encoder.finish()]);
        Submitted {
            output,
            staging,
            back,
            index,
        }
    }

    /// Waits for a dispatch's work (`index`, or all so far), and reads what
    /// it left in `staging`: `back` bytes of results into `out`, then its
    /// stop flag.
    fn read_back(
        &self,
        staging: &wgpu::Buffer,
        back: u64,
        out: Option<&mut [u8]>,
        index: Option<wgpu::SubmissionIndex>,
    ) -> Result<(), GpuError> {
        self.wait_mapped(staging, index)?;
        let view = staging
            .slice(..)
            .get_mapped_range()
            .map_err(|_| GpuError::Unavailable)?;
        let at = back as usize;
        let stopped: Vec<u32> = view[at..at + 20]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        if let Some(out) = out {
            let len = out.len().min(back as usize);
            out[..len].copy_from_slice(&view[..len]);
        }
        drop(view);
        staging.unmap();
        if stopped[0] != 0 {
            return Err(GpuError::Stopped {
                code: stopped[1],
                a: stopped[2],
                b: stopped[3],
                line: stopped[4],
            });
        }
        Ok(())
    }
}

/// A dispatch in the GPU's queue.
struct Submitted {
    output: wgpu::Buffer,
    staging: wgpu::Buffer,
    /// The bytes of results copied to `staging`, ahead of the stop flag.
    back: u64,
    index: wgpu::SubmissionIndex,
}

/// The workgroups for `n` items, as a grid: at most 65,535 across.
fn dispatch(n: usize) -> (u32, u32) {
    let groups = (n as u32).div_ceil(WORKGROUP).max(1);
    let x = groups.min(65_535);
    (x, groups.div_ceil(x))
}

/// Opens the GPU and compiles `wgsl`, so a later `map` with it starts
/// at once.
pub fn prepare(wgsl: &str) {
    if let Some(gpu) = gpu() {
        gpu.pipeline(wgsl, false);
    }
}

/// `items` mapped by kernel `wgsl` into `out`, one 32-bit word
/// (little-endian) per result, with `captures` (one word each) for the
/// values its lambda captured. An item may be several words (the fields
/// of a tuple or record), as many for each. Neither side is copied more than the GPU
/// needs; what stays in GPU memory comes back for later `map`s. With
/// `stay`, results that fit one dispatch stay on the GPU only, not
/// written into `out`: `read` brings them back when they are needed.
/// With `in_place` too (nothing else holds the items), results as many
/// words as the items are written over them, in their own buffer.
pub fn map(
    wgsl: &str,
    items: Items,
    captures: &[u32],
    out: &mut [u8],
    stay: bool,
    in_place: bool,
) -> Result<Kept, GpuError> {
    let gpu = gpu().ok_or(GpuError::Unavailable)?;
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
                results: Some(Resident { buffer, words }),
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
    let resident = |buffer, words| Some(Resident { buffer, words });
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
    let gpu = gpu().ok_or(GpuError::Unavailable)?;
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

mod filter;
pub use filter::filter;
mod late;
mod later;
mod pipeline;
mod spare;
mod upload;
pub use late::{Stop, finish, finish_before, flags_so_far};
pub use later::{Later, reduce, reduce_later};

#[cfg(test)]
mod tests;
