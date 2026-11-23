// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A slice of a list in GPU memory, made there (spec 15.4): one kernel
//! copies item `start + i * step` of the list into item `i` of the
//! slice, so nothing comes back to the CPU. The ends and the step are
//! worked out on the CPU first, as `kavach_runtime_core::slices` does.

use crate::{GpuError, Resident};
use std::sync::Arc;

/// The kernel for items of `width` words. `info` holds the slice's
/// length, its first item and its step; a negative step wraps as a
/// `u32`, and `start + i * step` wraps back into the list.
fn kernel(width: usize) -> String {
    let mut text = String::from(
        "// work 1\n// loops end\n\
@group(0) @binding(0) var<storage, read> items: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@group(0) @binding(2) var<storage, read> info: array<u32>;
@group(0) @binding(3) var<storage, read_write> stop: array<atomic<u32>, 5>;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
    let i = id.x + id.y * groups.x * 256u;
    if (i >= info[0]) { return; }
    let k = info[1] + i * info[2];
",
    );
    for j in 0..width {
        text.push_str(&format!(
            "    out[i * {width}u + {j}u] = items[k * {width}u + {j}u];\n"
        ));
    }
    text.push_str("}\n");
    text
}

/// The `n` items of `items` from `start`, every `step`th, each `width`
/// words, as a new list in the same GPU's memory. It doesn't wait for
/// the GPU: the copy can't stop.
pub fn slice(
    items: &Resident,
    start: usize,
    step: i64,
    n: usize,
    width: usize,
) -> Result<Resident, GpuError> {
    let gpu = items.gpu;
    let wgsl = kernel(width);
    let pipeline = gpu.pipeline(&wgsl, false);
    // With one item the step is never used; otherwise it is less than the
    // list's length, which fits a `u32`.
    let step = if n > 1 { step as i32 as u32 } else { 0 };
    let words = n * width;
    let captures = [start as u32, step];
    let buffer = gpu.run(
        &pipeline,
        &items.buffer,
        n,
        words,
        &captures,
        None,
        Some(&wgsl),
        None,
    )?;
    Ok(Resident {
        buffer: Arc::new(buffer),
        words,
        gpu,
    })
}

#[cfg(test)]
mod tests {
    use crate::{Items, available, map, read};

    /// Copies each item as it is, into the layout every kernel follows.
    const SAME: &str = "
@group(0) @binding(0) var<storage, read> items: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@group(0) @binding(2) var<storage, read> info: array<u32>;
@group(0) @binding(3) var<storage, read_write> stop: array<atomic<u32>, 5>;
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
    let i = id.x + id.y * groups.x * 256u;
    if (i >= info[0]) { return; }
    out[i] = items[i];
}
";

    /// Slices of a list in GPU memory, forwards and backwards, of items
    /// of one word and of three, on a real GPU: skipped where there is
    /// none.
    #[test]
    fn a_slice_is_made_in_gpu_memory() {
        if !available() {
            eprintln!("no GPU: skipped");
            return;
        }
        let n = 3000u32;
        let items: Vec<u8> = (0..n).flat_map(u32::to_le_bytes).collect();
        let mut out = vec![0u8; items.len()];
        let kept = map(SAME, Items::Bytes(&items), &[], &mut out, true, false).unwrap();
        let list = kept.results.expect("one dispatch");
        let words = |r: &crate::Resident, count: usize| {
            let mut b = vec![0u8; count * 4];
            read(r, &mut b).unwrap();
            let w: Vec<u32> = b
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from_le_bytes(*c))
                .collect();
            w
        };
        // `xs[10:20:3]`: 10, 13, 16, 19.
        let s = super::slice(&list, 10, 3, 4, 1).unwrap();
        assert_eq!(words(&s, 4), [10, 13, 16, 19]);
        // `xs[::-1000]`: 2999, 1999, 999.
        let s = super::slice(&list, 2999, -1000, 3, 1).unwrap();
        assert_eq!(words(&s, 3), [2999, 1999, 999]);
        // Items of three words, 1000 of them: `ys[#2:]`, then `ys[::-1]`.
        let s = super::slice(&list, 998, 1, 2, 3).unwrap();
        assert_eq!(words(&s, 6), [2994, 2995, 2996, 2997, 2998, 2999]);
        let s = super::slice(&s, 1, -1, 2, 3).unwrap();
        assert_eq!(words(&s, 6), [2997, 2998, 2999, 2994, 2995, 2996]);
    }

    #[test]
    fn the_kernel_copies_every_word_of_an_item() {
        let text = super::kernel(3);
        assert!(
            text.contains("out[i * 3u + 2u] = items[k * 3u + 2u];"),
            "{text}"
        );
        assert!(text.starts_with("// work 1\n// loops end\n"), "{text}");
    }
}
