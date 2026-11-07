# 0055. GPU results stay on the GPU until the CPU reads them

- **Status:** Accepted (2026-11-04, for review)
- **Date:** 2026-11-04
- **Builds on:** [decision 0044](0044-gpu-through-wgpu.md)

## Context

A GPU `map` copied its results back to the CPU every time. In a loop of `map`s (`xs = xs.map(step)`, a game's frame or a simulation's steps), most of the time went on that copy. The next `map` read the list from GPU memory, but the copy back was already made. A chain in one expression (`xs.map(f).map(g)`) already avoided it, as one kernel.

Mithlesh decided (2026-11-03) that this stays invisible: no `GpuList` type. The open question was when a panic is reported. If a `map` didn't wait for the GPU, its panic would show up later, at the next read.

## Decision

**A `map` whose result goes into a local leaves the result in GPU memory. The CPU fetches it at its first read.**

```kavach
for _ in 0..100 {
    xs = xs.map(x => step(x))   // stays on the GPU
}
print(xs[7])                    // the one copy back
```

- The compiler does it (`compiler/kir/src/gpu_stay.rs`). A local that a GPU `map` sets gets a fetch before every other read of it. The fetch is one call, and does nothing for a list already on the CPU. A `map` of the local reads it on the GPU, with no fetch.
- A loop that reads such a local but doesn't set it gets one fetch before the loop, not one each round.
- **Panics stay where they are.** A `map` still waits for the GPU, but reads back only the 20 bytes of its stop flag, not the list. A panic is reported at the `map`, with the line inside the `gpu fn`, as before.
- The runtime keeps up to four lists in GPU memory (`runtime/native/src/rt_gpu.rs`). To make room, it copies a list back first, unless nothing else holds it.
- A `map` the CPU runs (a short list, `KAVACH_GPU=off`, no GPU) fetches its list first. `KAVACH_GPU=check` copies every result back, to compare.

Measured, 10 million `Int32` items, a light `gpu fn` mapped 100 times: about 1,900 ms copying back each time, 530 ms now (3.6×). The CPU takes about 12 s.

## Alternatives considered

- **Check the list on every CPU read, at run time.** Every list read in native code would need a test, which slows down the code that never uses the GPU.
- **Don't wait for the GPU at the `map`.** This saves the wait, but a panic would be reported at a later line. Mithlesh wanted panics where they are, and the copy was the larger cost.
- **A `GpuList` type.** Rejected by Mithlesh: it should be invisible.

## Consequences

- A list read only on the GPU never comes back, which saves both the copy and the CPU memory traffic.
- Today the runtime's table of kept lists is per thread. Tasks don't move between threads while they run, so a fetch always finds its list. If tasks start to move (to-do item 3), the table must become shared first.
- `sum`, `min`, `max` and `filter` on the GPU (the next step of the GPU pipelines to-do item) can end such a pipeline without a copy of the whole list.
