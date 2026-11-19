# 0139. Several GPUs: the system chooses, and code can say where

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

Until now the runtime opened one GPU, the one `wgpu` called high-performance, and ran all GPU work there (spec 15.2). A machine often has more: a laptop has the processor's integrated GPU and a discrete card; a workstation may hold an NVIDIA and an AMD card; Windows adds a software GPU (WARP). `wgpu` lists every adapter of every vendor, so each can be a device. GPUs of different vendors share no memory: a list one holds reaches another only through main memory.

Spec 15 asked, as an open question, for a `Gpu` capability for code that chooses the device. Decision [0126](0126-loops-on-one-core-unless-on-cores.md) added `on gpu(n)` before a value and `gpu(n) fn`, numbered as `system.gpus()` lists them, and left them to this item (to-do 16).

How others do it:

- **CUDA:** `cudaSetDevice(n)` sets the device for the calling thread; `CUDA_VISIBLE_DEVICES` hides devices for one run. Devices are numbered from 0.
- **PyTorch:** a tensor lives on a device (`x.to("cuda:1")`), and `with torch.cuda.device(1):` sets the default in a block.
- **JAX:** `jax.devices()` lists them; `jax.default_device` in a `with` block; computations run where their data is.

## Decision

A maintainer chose (2026-11-15) the rules of spec 15.6:

1. **By default the system decides.** Devices are ranked at the first GPU work: discrete, then integrated, then the rest, the CPU last. New work goes to the least busy device, by Kavach's own count of the work it sent and hasn't finished. A device that fails (can't open, lost, out of memory) is skipped for the next, ending on the CPU.
2. **A big job splits across all GPUs** when its size pays for moving the data. This is the second phase, not built yet.
3. **A developer chooses** with a block, `gpu.on(device) { … }` (spec 4.8, its spawned tasks too), or an argument, `xs.map(f, on: device)`, which wins over the block; with `KAVACH_GPU` (a vendor, a number or `cpu`) for one run, and `[run] gpu` in `kavach.toml`, with no code change.
4. **`gpu.devices()`** lists them, with the data of `system.gpus()`. Choosing or listing needs `uses Gpu`; a plain `gpu fn` and its `map` stay free.

Building it (2026-11-19) settled the details the choices left open. Each is easy to change, and is marked for a maintainer's review in the to-do:

- **Numbers start at 0,** as list indexes do: `on gpu(0)` is `system.gpus()[0]`, the best device.
- **The nearest choice wins:** a call's `on:`, then the innermost `gpu.on`, `on gpu(n)` or `on cpu`, then `gpu(n) fn`, then `KAVACH_GPU`, then `[run] gpu`, then the system. A choice made where the code calls wins over one made where the function is written. `KAVACH_GPU=cpu` still keeps everything on the CPU, as a debugging switch.
- **A choice says where, not whether:** a `map` too small to pay for the GPU still runs on the CPU (15.2).
- **A software GPU runs work only when chosen.** WARP and llvmpipe draw on the processor, where Kavach's own code is faster; ranked last, they would only slow automatic work down.
- **A list stays where it is:** with no choice, work on a list already in one GPU's memory runs on that GPU, however busy, rather than copying the list over.
- **`[run] gpu` is the one setting:** it takes a vendor's name or a number besides `"cpu"` and `"auto"`, instead of a new `[gpu]` section.
- **A missing GPU stops the program at its start, with exit code 1,** for every number written in the code (`gpu(n) fn` and `on gpu(n)`) and for `KAVACH_GPU` and `[run] gpu`. A number worked out while running is a panic where it is used.
- **A map of a `gpu(n) fn` needs `uses Gpu`** in the function that calls `map`, as the `gpu fn` itself can use no capability (15.1).
- **Drawing stays on the program's GPU:** the device `KAVACH_GPU` or `[run] gpu` names, or the best. A list on another device is copied over.

The runtime keeps one table of devices (`runtime/gpu/src/devices.rs`), listed once, so a number means the same device all run long; each device is opened when work first goes to it. `KAVACH_FAKE_GPUS` (spec 14.25) stands in for the list in tests, and a fake entry with a real GPU's name opens that GPU, so choosing is tested with Windows' WARP as a real second device.

## Consequences

- A program uses a machine's best GPU with no code, and a second one when the first is busy.
- A developer can pin work to a device, and an operator can choose one for a run, without changing the code.
- A device that disappears or fails doesn't stop the program: the work moves on, with the same results.
- Each device opened costs its own start-up time and memory, and compiles its own kernels. Only devices that get work are opened.
- Listing every interface (Vulkan, DirectX 12, Metal, OpenGL) at the first GPU work costs more than opening one GPU through Vulkan did.

## Alternatives considered

- **Numbers from 1.** `gpu(1)` would read as "the first GPU", but `system.gpus()[1]` is the second: two numberings for one list.
- **`gpu(n) fn` winning over the caller's choice.** The function's author can't know the machine; the caller, or the person running it, knows more.
- **A choice forcing the GPU for any size.** `KAVACH_GPU=on` already does that for debugging; a choice in the code shouldn't make small maps slower.
- **Devices as values a list lives on, as PyTorch does.** Kavach's lists move between the CPU and the GPU unseen (15.2); a device in a list's type would make that visible everywhere.
