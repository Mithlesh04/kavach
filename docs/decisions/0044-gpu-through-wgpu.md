# 0044. GPU code: `gpu fn`, run by `map`, through `wgpu`

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

Mithlesh asked for GPU code that is easy to write and at least 20× faster (2026-11-02), and for it to be built, not only designed (2026-11-03). The research is in [architecture/gpu.md](../architecture/gpu.md). Its findings:

- The pain is moving memory by hand, launch boilerplate, indices that go out of bounds, races, no debugger, two languages, and vendor lock-in.
- The model that avoids most of it is data-parallel operations over whole collections, in the host language, compiled by the toolchain.
- GPUs are fast at 32-bit arithmetic. A copy over PCIe costs more than light work, so only compute-heavy work gains 20×.

Mithlesh approved `wgpu` as the dependency (2026-11-03).

## Decision

**A function marked `gpu fn` can run on the GPU. `list.map(f)` runs on the GPU when `f` is one, or a lambda that calls one, and the list is long enough (spec chapter 15).**

1. A `gpu fn` takes and gives `Float32`, `Int32`, `UInt32` or `Bool`, and its body uses only what the GPU can do. The checker enforces this, so a mistake is a compile error, never a GPU failure.
2. There are no indices, buffers or launches in the code. The compiler translates `gpu fn`s to WGSL. The runtime uploads, dispatches, reads back and frees, and caches each pipeline.
3. The same code runs on the CPU, so the interpreter, `print` and debuggers work on it. Native code uses the CPU when the list is short, when there is no GPU, or when `KAVACH_GPU=off` is set.
4. Integer overflow and division by zero panic as on the CPU. Float results match the CPU's but for a fused multiply-add, which can change the last bit.
5. `wgpu` runs on Vulkan, DirectX 12 and Metal, so NVIDIA, AMD, Intel and Apple GPUs all work. It lives in `runtime/gpu`, behind its own runtime entry (decision 0024), so programs without GPU code don't link it.

## Consequences

- GPU code is ordinary Kavach, type-checked like the rest, with no new syntax beyond `gpu`.
- Compute-heavy `map`s gain the GPU's speed. Light ones stay on the CPU, which is faster for them.
- Keeping data on the GPU between steps (`GpuList<T>`) and GPU `sum`, `filter` and `zip` come next; they reuse this machinery.

## Alternatives considered

- **CUDA through `cudarc`:** the fastest on NVIDIA, but NVIDIA only; Mithlesh chose `wgpu`.
- **Kernels with explicit indices and buffers** (CUDA, WGSL by hand): full control, but it brings back every pain the research lists.
- **Running any lambda on the GPU automatically:** no marker needed, but a function that can't run there would silently stay on the CPU, and the user wouldn't know why it's slow. With `gpu fn` the checker says what's wrong.
- **SPIR-V directly** (as `platform.md` first planned): `wgpu` takes WGSL on every backend and translates it itself, so Kavach doesn't need its own SPIR-V writer.
