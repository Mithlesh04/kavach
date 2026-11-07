# runtime/gpu/

GPU compute for `gpu fn` (spec [chapter 15](../../docs/spec/15-gpu.md), [decision 0044](../../docs/decisions/0044-gpu-through-wgpu.md)), through `wgpu`: one device for the program, a pipeline compiled once per kernel, and `map` over 32-bit items. Native code reaches it through `kv_rt_gpu` (`runtime/native/src/rt_gpu.rs`), so programs without GPU code don't link it (decision 0024). The kernels' WGSL comes from `compiler/backend/wgsl`.

| File | Job |
|---|---|
| [src/lib.rs](src/lib.rs) | Caching pipelines, running a `map` in chunks the device can hold, leaving results in GPU memory, `read` |
| [src/later.rs](src/later.rs) | `reduce`: a `sum`, `min` or `max` in the GPU's queue, whose partials are read when they are needed |
| [src/device.rs](src/device.rs) | Opening the program's one device, Vulkan first |
| [src/filter.rs](src/filter.rs) | `filter`: the kernel's two runs over the same items, and where each workgroup's kept items start |
| [src/late.rs](src/late.rs) | The stop flags of `map`s that didn't wait for the GPU, read later (decision 0061) |
| [src/spare.rs](src/spare.rs) | Buffers of items and results no list holds any more, reused by the next `map` of the same size |
| [src/tests.rs](src/tests.rs) | A kernel on a real GPU: results, results left there and read back, a stop (skipped without a GPU) |
