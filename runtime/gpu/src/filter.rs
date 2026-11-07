//! `filter` on the GPU (spec 15.2): the kernel from
//! `compiler/backend/wgsl/src/filter.rs`, run twice over the same items.
//! The first run counts each workgroup's kept items; their running total
//! says where each group writes; the second run writes them there, in
//! order.

use crate::GpuError;
use crate::{Items, Kept, Resident, dispatch, gpu};
use std::sync::Arc;

/// The items of `items` (`n` one-word items) kernel `wgsl` keeps, with
/// `captures` for its lambda: how many, what stays in GPU memory, and
/// their words, unless `stay` leaves them only there.
pub fn filter(
    wgsl: &str,
    items: Items,
    n: usize,
    captures: &[u32],
    stay: bool,
) -> Result<(usize, Kept, Option<Vec<u8>>), GpuError> {
    let gpu = gpu().ok_or(GpuError::Unavailable)?;
    if n == 0 || n > gpu.max_items {
        return Err(GpuError::Unavailable);
    }
    let pipeline = gpu.pipeline(wgsl, false);
    let (input, sent) = match items {
        Items::Resident(r) => (r.buffer.clone(), None),
        Items::Bytes(b) => {
            let buffer = Arc::new(gpu.load(b));
            let sent = Resident {
                buffer: buffer.clone(),
                words: n,
            };
            (buffer, Some(sent))
        }
    };
    let (x, y) = dispatch(n);
    let groups = (x * y) as usize;
    // The first run: each group's count.
    let mut info = captures.to_vec();
    info.push(0);
    let mut counts = vec![0u8; groups * 4];
    gpu.run(
        &pipeline,
        &input,
        n,
        groups,
        &info,
        Some(&mut counts),
        None,
        None,
    )?;
    info.pop();
    info.push(1);
    let mut total = 0u32;
    for c in counts.as_chunks::<4>().0 {
        info.push(total);
        total += u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
    }
    // The second: the kept items, where their group starts.
    let len = total as usize;
    let mut out = (!stay).then(|| vec![0u8; len * 4]);
    let results = gpu.run(
        &pipeline,
        &input,
        n,
        len.max(1),
        &info,
        out.as_deref_mut(),
        None,
        None,
    )?;
    let kept = Kept {
        items: sent,
        results: Some(Resident {
            buffer: Arc::new(results),
            words: len,
        }),
    };
    Ok((len, kept, out))
}
