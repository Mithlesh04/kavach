# runtime/gpu/

GPU compute for `gpu fn` (spec [chapter 15](../../docs/spec/15-gpu.md), [decision 0044](../../docs/decisions/0044-gpu-through-wgpu.md)), through `wgpu`: one device for the program, a pipeline compiled once per kernel, and `map` over 32-bit items. Native code reaches it through `kv_rt_gpu` (`runtime/native/src/rt_gpu.rs`), so programs without GPU code don't link it (decision 0024). The kernels' WGSL comes from `compiler/backend/wgsl`.

| File | Job |
|---|---|
| [src/lib.rs](src/lib.rs) | Caching pipelines, running a `map` in chunks the device can hold, leaving results in GPU memory, `read` |
| [src/later.rs](src/later.rs) | `reduce`: a `sum`, `min` or `max` in the GPU's queue, whose partials are read when they are needed |
| [src/device.rs](src/device.rs) | Opening the program's one device, Vulkan first, keeping its instance and adapter for surfaces |
| [src/listing.rs](src/listing.rs) | Listing every GPU for `system.gpus()` (spec 14.25), best kind first, without opening one; vendors by PCI ID; `KAVACH_FAKE_GPUS` in its place for tests |
| [src/draw.rs](src/draw.rs) | Drawing into windows for `std.gpu` (spec 14.24, decision 0106): a screen is a surface made on the window's own thread, sized at each frame; frames cleared and shown, by number |
| [src/render.rs](src/render.rs) | Pipelines compiled ahead (when a screen opens, or when made once one is open), with depth or not, and draws from 32-bit words into a frame (spec 15.3) |
| [src/picture.rs](src/picture.rs) | `picture`: draws into a texture off the screen, read back, for tests (decision 0106) |
| [src/texture.rs](src/texture.rs) | Textures by number, the one sampler, their bind group, and depth buffers (spec 15.3) |
| [src/ktx2.rs](src/ktx2.rs) | KTX2 files of compressed blocks (BC1 to BC7, ASTC 4×4) read with no library: the format, the size and each mip level's bytes, which `texture::compressed` uploads as they are (spec 14.24) |
| [src/filter.rs](src/filter.rs) | `filter`: the kernel's two runs over the same items, and where each workgroup's kept items start |
| [src/late.rs](src/late.rs) | The stop flags of `map`s that didn't wait for the GPU, read later (decision 0061) |
| [src/pipeline.rs](src/pipeline.rs) | Compiling each kernel into a pipeline once; a kernel whose loops surely end goes without `wgpu`'s loop counter ([decision 0077](../../docs/decisions/0077-kernels-whose-loops-end.md)) |
| [src/spare.rs](src/spare.rs) | Buffers of items and results no list holds any more, reused by the next `map` of the same size |
| [src/upload.rs](src/upload.rs) | Items of 1 MB or more copied over through two buffers of our own, used in turn: `write_buffer` made a new 64 MB staging buffer every few chunks of a stream, at 100–280 ms each |
| [src/timers.rs](src/timers.rs) | The GPU's own time for a frame's drawing and each `map`, from timestamp queries, when a trace or Tracy is on (spec 14.12, decision 0123); read back without waiting |
| [src/tests.rs](src/tests.rs) | A kernel on a real GPU: results, results left there and read back, a stop, big items through the upload buffers (skipped without a GPU) |
