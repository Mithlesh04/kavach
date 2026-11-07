# 0067. Streaming data through the GPU

- **Status:** Accepted (2026-11-05, Mithlesh chose the recommendation: option 1)
- **Date:** 2026-11-04
- **Changes:** spec 15; `runtime/gpu`, `std.fs`

## Context

The to-do's item 4 (Mithlesh, 2026-11-03): data that arrives from the network, files or an API should be worked on by the GPU in chunks, without waiting on copies. Mithlesh wants the options before a choice. Today a GPU `map` over each chunk of `fs.chunks(path)` copies the chunk over, waits for the GPU, and copies the result back, one chunk after another. The disk, the bus and the GPU each wait for the others.

Three parts are needed whatever the choice:

1. **Numbers read from `Bytes` without a copy**, so a chunk from `fs.chunks` or a socket is seen as a `[Float32]`. Today only `b.to_list()` exists, which gives a `[UInt8]`.
2. **Copies that overlap the work.** While the GPU works on chunk n, chunk n+1 goes over and chunk n−1 comes back. This takes two or three buffers in turn, in memory the system keeps in place ("pinned").
3. **Sums and other totals kept on the GPU** across chunks until the end.

## What others do

- **CUDA** makes the program do it: streams, a ring of pinned buffers, and events to know when each chunk is done. Overlapping gives about 1.8× on an RTX 4090 in NVIDIA's own example ([cuda-oxide](https://nvlabs.github.io/cuda-oxide/async-programming/overlapping-transfers-and-compute.html)), but each program writes the ring itself.
- **PyTorch** hides it in the data loader: `DataLoader(pin_memory=True)`, then `.to("cuda", non_blocking=True)`, and a loop over batches. The loader fetches the next batch while the GPU works on this one.
- In both, copying a result back to the CPU each round stops the overlap.

## Options

1. **Invisible, like the rest of Kavach's GPU work** (decision 0055, Mithlesh's "invisible, no `GpuList` type"). A `for chunk in fs.chunks(path)` loop whose body maps the chunk on the GPU is overlapped by the runtime, as PyTorch's loader does, with nothing new to write. `Bytes` gains `b.as_list<Float32>()`, which views the bytes without copying them.
   - Nothing new to learn.
   - It works only for the loop shapes the compiler recognizes, and a program can't tell whether its loop was overlapped (`kavach check` can say so as a note).
2. **A stream type**, `gpu.stream(source, f)`, which gives the results chunk by chunk, overlapped.
   - The program says what it wants, so it always gets it.
   - It is a second way to use the GPU, against Mithlesh's "invisible" rule.
3. **Both**: the invisible loop, plus a function for the cases the compiler can't see.

## Recommendation

Option 1, as it keeps Mithlesh's rule. `kavach check` notes when such a loop is not overlapped and why. Measure it on a file larger than GPU memory, against C++ with CUDA streams (to-do item 4).

## Decision

Mithlesh chose option 1 (2026-11-05). Why:

- **One way to use the GPU.** Every GPU feature so far is invisible: a `map` picks the GPU by its cost (spec 15), lists stay in GPU memory without a `GpuList` type (decision 0055), and frames don't wait for each `map` (decision 0061). A stream type would be a second way, which a reader must learn and a writer must choose between.
- **The common case needs no code.** A loop over a file's chunks is how a program reads big data anyway. As with PyTorch's data loader, the program that already works gets faster; with CUDA's streams, each program writes its own ring of buffers and gets it wrong in its own way.
- **The program is never wrong, only slower.** The results are the same overlapped or not; only the time differs. So recognizing too few loop shapes costs speed, not correctness, and `kavach check`'s note says which loop missed and why, so the writer can reshape it.
- **A function can still come later.** If programs show shapes the compiler can't see, option 3's function can be added then, from those programs. Starting with it would fix its design before anyone needs it.

## Open questions

- How many buffers in turn (two or three), and how big a chunk the runtime asks for by itself.
- Whether network chunks (`receive.take()` of `Bytes`) are overlapped the same way, or files first.
