# 0144. One big map across several GPUs

- **Status:** Accepted (points marked "for a maintainer's review" are open to change)
- **Date:** 2026-11-23

## Context

[Decision 0139](0139-choosing-gpus.md) chose that a big job splits across all GPUs "when its size pays for moving the data", with results identical, as its second phase. The first phase sends each piece of work to one device: the least busy, or the one the code or a setting chooses. This record settles how the second phase works.

GPUs of different vendors share no memory, so each part's items cross from main memory to its device, and its results come back. A list one device holds can't be read by another without coming back first.

How others do it:

- **PyTorch's `DataParallel`** cuts a batch into equal parts, one for each GPU, runs them at once and gathers the results on one GPU. Unequal GPUs make it wait for the slowest.
- **JAX's `pmap` and sharding** cut along one axis into equal parts; the programmer says how.
- **StarPU and OmpSs** (research runtimes for mixed CPUs and GPUs) time each device on each kernel and give it work in proportion to its measured speed.

## Decision

Spec 15.6, "A big job splits across the GPUs":

1. **Only a `map` splits**, and only when nothing chooses its device, its items are in main memory, two or more devices may take automatic work and are open, and its size pays. `filter`, `sum`, `min` and `max` run on a list one GPU already holds (15.2), so they stay there. A matrix product runs on the CPU today (14.33), so it can't split yet.
2. **Its size pays** when the work would take the CPU at least a second, and is at least 100 units for each 32-bit word of an item and its result. The units are the compiler's estimate of a `gpu fn`'s work (15.2), about 0.1 ns of the CPU each. The cost a split adds is small next to that: the parts' results always come back to main memory, where one device's could have stayed, and each part is one more dispatch and read back. The CPU's time is only a rough guide to a GPU's, so once every device has a measured speed (point 3), the measure decides: a `map` splits only when its parts would end at least a tenth sooner than the fastest device alone, with 2 ms added for each part's thread, dispatch and read back. On the test laptop, an RTX 4050 did 4 million items of 4,000 steps in about 10 ms, where the CPU's estimate said several seconds: an Intel UHD 770 beside it made the job slower, not faster, and the measure stops such splits after the first. *For a maintainer's review:* the three numbers, and that a device's speed is measured only on parts of splits, so a split stopped by the measure isn't tried again.
3. **One part for each device, in the ranking's order, sized to its speed.** At first the speeds are guessed by kind: a discrete GPU counts as 8 integrated ones (an RTX 4050 laptop GPU against an Intel UHD on one laptop is about that or more). Once each device has done a part, its measured speed (the part's work over its time, the kernel compiled before the clock starts) is used, averaged with the last one. A device whose part would be under 256 items (one workgroup) is left out. *For a maintainer's review:* the factor 8, and averaging with the last measure only.
4. **The parts run at once**, one thread for each device, and write their results into their own run of the one result list. So the order is the list's, and each item's result is what one device gives it (15.2): exact sums of the results don't change in any split (15.5).
5. **A panic** in a part stops the work with its message; when several parts stop, the first part's is reported, so the item is the earliest one a GPU saw failing. *For a maintainer's review.*
6. **A device that fails during its part** is skipped from then on (15.6), and its part runs on the next device by the usual choice, or the whole `map` on the CPU when none is left.
7. **A device not open yet is opened in the background** and joins later work, rather than holding this one up: opening a GPU takes 100–500 ms, more than the GPU time of most maps. *For a maintainer's review.*
8. **`KAVACH_GPU_SPLIT`**, for tests and comparisons: `on` splits every `map` that may split, at any size, opening its devices first; `off` never splits. *For a maintainer's review.*

## Consequences

- A machine with two GPUs runs one big `map` on both, with no code and the same results.
- Unequal GPUs share the work by what they can do, once each has been measured; the first split's guess by kind may be off.
- A split's results come back to main memory, so a pipeline of maps on a split list copies each list over and back. Heavy work per item pays for that; light work stays on one device.
- Each device that takes parts is opened, with its memory and its kernels compiled.

## Alternatives considered

- **Equal parts.** The slowest device sets the time: an integrated GPU given half a job would make it slower than the discrete GPU alone.
- **A split list, each part kept in its device's memory** for the next `map`. Chained work would never come back between steps, but every later operation (`filter`, `sum`, a read, drawing) would have to know a list can live on several devices. A later step, if measured work needs it.
- **Waiting for every device to open.** The first big `map` would wait for a GPU that does a small part of it.
- **Splitting `filter` and sums too.** They run only on a list one GPU holds; moving it to split it costs more than they save.
