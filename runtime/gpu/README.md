# runtime/gpu/

GPU compute for `gpu fn` (spec [chapter 15](../../docs/spec/15-gpu.md), [decision 0044](../../docs/decisions/0044-gpu-through-wgpu.md)), through `wgpu`: a table of every device, the least busy or the chosen one running each piece of work (spec 15.6), a pipeline compiled once per kernel and device, and `map` over 32-bit items. Native code reaches it through `kv_rt_gpu` (`runtime/native/src/rt_gpu.rs`), so programs without GPU code don't link it (decision 0024). The kernels' WGSL comes from `compiler/backend/wgsl`.

| File | Job |
|---|---|
| [src/lib.rs](src/lib.rs) | Running a dispatch and reading its results back, leaving results in GPU memory |
| [src/later.rs](src/later.rs) | `reduce`: a `sum`, `min` or `max` in the GPU's queue, whose partials are read when they are needed |
| [src/device.rs](src/device.rs) | Opening one device, keeping its instance and adapter for surfaces; its count of work not finished, and a watch for its loss or running out of memory |
| [src/devices.rs](src/devices.rs) | The program's devices (spec 15.6): one table listed once, each opened when work first goes to it, skipped once it fails; `pick` for each piece of work, the program's GPU for drawing, and the start's check |
| [src/split.rs](src/split.rs) | Splitting one big `map` across the GPUs (spec 15.6, decision 0144), as plain functions tested on any machine: whether its size pays, each device's speed (guessed by kind, then measured), and the run of items each takes |
| [src/choice.rs](src/choice.rs) | Which device gets the work, as plain functions tested on any machine: the least busy, a chosen one then the rest, a `gpu(n) fn`'s kernel line, `KAVACH_GPU` and `[run] gpu`, the start's messages |
| [src/map.rs](src/map.rs) | `map` and `read`, and running work on the chosen device with its items moved there through main memory, the next device when one is lost; a split `map`'s parts at once, one thread for each device |
| [src/listing.rs](src/listing.rs) | Listing every GPU for `system.gpus()` (spec 14.25), best kind first, without opening one; two of one model told apart by their bus; vendors by PCI ID; `KAVACH_FAKE_GPUS` in its place for tests |
| [src/draw.rs](src/draw.rs) | Drawing into windows for `std.gpu` (spec 14.24, decision 0106): a screen is a surface made on the window's own thread, sized at each frame; frames cleared and shown, by number |
| [src/render.rs](src/render.rs) | Pipelines compiled ahead (when a screen opens, or when made once one is open), with depth or not, and draws from 32-bit words into a frame (spec 15.3) |
| [src/picture.rs](src/picture.rs) | `picture`: draws into a texture off the screen, read back, for tests (decision 0106) |
| [src/texture.rs](src/texture.rs) | Textures by number, the one sampler, their bind group, and depth buffers (spec 15.3) |
| [src/ktx2.rs](src/ktx2.rs) | KTX2 files of compressed blocks (BC1 to BC7, ASTC 4×4) read with no library: the format, the size and each mip level's bytes, which `texture::compressed` uploads as they are (spec 14.24) |
| [src/slice.rs](src/slice.rs) | `slice`: a slice of a list in GPU memory made there, item `start + i * step` copied into item `i` (spec 15.4) |
| [src/filter.rs](src/filter.rs) | `filter`: the kernel's two runs over the same items, and where each workgroup's kept items start |
| [src/late.rs](src/late.rs) | The stop flags of `map`s that didn't wait for the GPU, read later (decision 0061) |
| [src/pipeline.rs](src/pipeline.rs) | Compiling each kernel into a pipeline once; a kernel whose loops surely end goes without `wgpu`'s loop counter ([decision 0077](../../docs/decisions/0077-kernels-whose-loops-end.md)) |
| [src/spare.rs](src/spare.rs) | Buffers of items and results no list holds any more, reused by the next `map` of the same size |
| [src/upload.rs](src/upload.rs) | Items of 1 MB or more copied over through two buffers of our own, used in turn: `write_buffer` made a new 64 MB staging buffer every few chunks of a stream, at 100–280 ms each |
| [src/timers.rs](src/timers.rs) | The GPU's own time for a frame's drawing and each `map`, from timestamp queries, when a trace or Tracy is on (spec 14.12, decision 0123); read back without waiting |
| [src/tests.rs](src/tests.rs) | A kernel on a real GPU: results, results left there and read back, a stop, big items through the upload buffers (skipped without a GPU); what listing and opening the GPUs costs, timed by hand |
