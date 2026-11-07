//! The GPU runtime on a real GPU: skipped where there is none.

use super::*;

/// The buffer layout every kernel follows, doubling each item, and
/// stopping (code 7) at item 5 when asked to.
const DOUBLE: &str = "
@group(0) @binding(0) var<storage, read> items: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@group(0) @binding(2) var<storage, read> info: array<u32>;
@group(0) @binding(3) var<storage, read_write> stop: array<atomic<u32>, 5>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
let i = id.x + id.y * groups.x * 256u;
if (i >= info[0]) { return; }
if (info[1] == 1u && i == 5u && atomicExchange(&stop[0], 1u) == 0u) {
    atomicStore(&stop[1], 7u);
    atomicStore(&stop[2], i);
}
out[i] = items[i] * 2u;
}
";

#[test]
fn a_kernel_runs_and_stops_like_the_cpu() {
    if !available() {
        eprintln!("no GPU: skipped");
        return;
    }
    let items: Vec<u8> = (0..100_000u32).flat_map(u32::to_le_bytes).collect();
    let mut doubled = vec![0u8; items.len()];
    let kept = map(
        DOUBLE,
        Items::Bytes(&items),
        &[0],
        &mut doubled,
        false,
        false,
    )
    .unwrap();
    let word = |b: &[u8], i: usize| u32::from_le_bytes(b[4 * i..4 * i + 4].try_into().unwrap());
    assert!((0..100_000).all(|i| word(&doubled, i) == 2 * i as u32));
    // The results, left on the GPU, doubled again without copying.
    let mut quadrupled = vec![0u8; items.len()];
    let results = kept.results.expect("one dispatch");
    map(
        DOUBLE,
        Items::Resident(&results),
        &[0],
        &mut quadrupled,
        false,
        false,
    )
    .unwrap();
    assert!((0..100_000).all(|i| word(&quadrupled, i) == 4 * i as u32));
    // Results that stay: nothing comes back until `read`.
    let mut later = vec![0u8; items.len()];
    let kept = map(
        DOUBLE,
        Items::Resident(&results),
        &[0],
        &mut later,
        true,
        false,
    )
    .unwrap();
    assert!(later.iter().all(|b| *b == 0));
    read(&kept.results.expect("one dispatch"), &mut later).unwrap();
    assert!((0..100_000).all(|i| word(&later, i) == 4 * i as u32));
    // Its buffer, let go of after the read, is the next one's: written
    // over in full.
    let kept = map(DOUBLE, Items::Bytes(&items), &[0], &mut later, true, false).unwrap();
    read(&kept.results.expect("one dispatch"), &mut later).unwrap();
    assert!((0..100_000).all(|i| word(&later, i) == 2 * i as u32));
    // Written over its own items, when nothing else holds them.
    let kept = map(
        DOUBLE,
        Items::Resident(&results),
        &[0],
        &mut later,
        true,
        true,
    )
    .unwrap();
    let over = kept.results.expect("one dispatch");
    assert!(Arc::ptr_eq(&over.buffer, &results.buffer));
    read(&over, &mut later).unwrap();
    assert!((0..100_000).all(|i| word(&later, i) == 4 * i as u32));
    let stopped = map(
        DOUBLE,
        Items::Bytes(&items),
        &[1],
        &mut doubled,
        false,
        false,
    )
    .map(|_| ());
    assert_eq!(
        stopped,
        Err(GpuError::Stopped {
            code: 7,
            a: 5,
            b: 0,
            line: 0
        })
    );
    // One that stays doesn't wait: its stop is read at `finish`
    // (decision 0061).
    let late = map(
        DOUBLE,
        Items::Bytes(&items),
        &[1],
        &mut doubled,
        true,
        false,
    );
    assert!(late.is_ok());
    assert_eq!(finish(), Some((7, 5, 0, 0, DOUBLE.to_string())));
    assert_eq!(finish(), None);
}
