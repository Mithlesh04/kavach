# GPU Computing: What Makes It Hard, and How Kavach Makes It Easy

Research done 2026-11-03 for the to-do item "GPU code that is easy to write and at least 20× faster". It covers what makes GPU programming hard today, how other projects ease it, and the design that follows for Kavach. The language rules are in spec [chapter 15](../spec/15-gpu.md), and the choice of library is in [decision 0044](../decisions/0044-gpu-through-wgpu.md).

## Why a GPU, and when it is not faster

A GPU runs thousands of simple threads at once. The test machine's NVIDIA RTX 4050 Laptop GPU has 2,560 shader cores, about 12 TFLOPS of 32-bit float arithmetic, and 192 GB/s of memory bandwidth on 6 GB of its own memory. One laptop CPU core does a few tens of GFLOPS with AVX2, so all of its cores together do several hundred.

Three facts decide whether the GPU wins:

1. **64-bit floats are slow.** Consumer NVIDIA GPUs run `f64` at 1/64 of the `f32` rate, and WGSL, WebGPU's shading language, has no `f64` at all in its core ([gpuweb discussion](https://lists.w3.org/Archives/Public/public-gpu/2022Apr/0006.html)). Metal has none either. So GPU code works on `Float32`, `Int32`, `UInt32` and `Bool`. For that, Kavach's native code first had to get a real `Float32` (spec 2.1).
2. **Moving data costs more than simple work.** A discrete GPU sits behind PCIe, at about 16 to 25 GB/s each way on a laptop, while the CPU reads its own memory at 50 to 100 GB/s. So `xs.map(x => x * 2.0)` is *slower* on the GPU once the copies are counted. Only work with many operations per item (simulations, image filters, fractals, physics, statistics with heavy math) gains 20× or more. The same holds for data that stays on the GPU across several steps.
3. **Starting the GPU has a fixed cost.** Each dispatch costs tens of microseconds, and reading results back costs a millisecond or more ([readback measurements](https://forum.babylonjs.com/t/webgpu-compute-shader-babylon-js-vs-plain-js-speed-comparison/49448)). Compiling a pipeline costs much more, so pipelines must be cached ([vyre-driver-wgpu](https://docs.rs/crate/vyre-driver-wgpu/latest/source/src/pipeline.rs)).

So "20× faster in every benchmark it targets" is honest only for compute-heavy work. Kavach runs small or light work on the CPU and says so in its docs, instead of claiming a speedup the hardware can't give.

## What makes GPU programming hard today

| Pain | Where it hurts | What it costs |
|---|---|---|
| **Moving memory by hand** | CUDA `cudaMalloc` and `cudaMemcpy`, OpenCL buffers, Vulkan memory types and staging buffers, WebGPU staging buffers and `mapAsync` | The most common source of slowness and bugs ([developer guides](https://moldstud.com/articles/p-what-are-some-common-challenges-faced-by-cuda-developers)): copies forgotten, done twice, or leaked |
| **Launch boilerplate** | Vulkan compute needs hundreds of lines (instance, device, queues, descriptor sets, pipeline layouts) before the first kernel; WebGPU fewer, CUDA few | New users give up before running anything |
| **Thread indices and bounds** | Every CUDA, OpenCL, Metal and WGSL kernel computes its own index and must check `i < n` | A missed check writes outside the buffer: corrupt results, "device lost", or a driver reset (Windows kills GPU work that runs over 2 seconds) |
| **Races and synchronization** | Shared memory, barriers, atomics | Wrong answers that show up one run in a thousand |
| **Debugging** | `printf`, `cuda-gdb` and Nsight for CUDA; nearly nothing for WebGPU; errors arrive late, asynchronously | Hours per bug; many developers debug by guessing |
| **Two languages** | Host code in C++, Python or JavaScript; kernels in CUDA C++, WGSL, MSL or GLSL | Types are declared twice, and drift apart |
| **Vendor lock-in** | CUDA runs on NVIDIA only, Metal on Apple only; ROCm covers few AMD cards; OpenCL is deprecated on Apple | Code is rewritten per vendor, or locked to one |
| **Silent arithmetic** | Integer overflow wraps, division by zero gives an unspecified value, and fast-math reorders float operations | GPU results differ from the CPU's, and no one notices |
| **Tuning** | Block or workgroup sizes, occupancy, memory coalescing | Experts get 10× more than beginners from the same code |
| **Compile times and stutter** | PTX JIT, shader compilation at first use | Pauses the first time a kernel runs |

Academic work on CUDA performance problems ([Bi et al., FSE 2026](https://conf.researchr.org/details/fse-2026/fse-2026-research-papers/44/Understanding-Performance-Problems-in-CUDA-Programs)) finds the same themes: memory transfers and access patterns dominate.

## How others make it easier

- **Triton** (OpenAI, 2021) works on *blocks* of data instead of threads, and handles coalescing, shared memory and synchronization itself ([OpenAI](https://openai.com/index/triton/)). Researchers write fast kernels without CUDA expertise, though some GPU knowledge is still needed.
- **NumPy-style arrays on the GPU** (JAX with XLA, CuPy, PyTorch) express work as whole-array operations, with no indices, and fuse chains of them into one kernel. This is the most widely used model, because users write what they mean.
- **Futhark, Halide and Taichi** show that a restricted, data-parallel language can be compiled to fast GPU code safely. Halide also separates *what* is computed from *how* it is scheduled.
- **Mojo, rust-gpu and CubeCL** put kernels in the same language as the host, which removes the two-language problem.

The common lesson: **data-parallel operations over whole collections, with no indices, in the host language, compiled by the toolchain**, avoid most of the pain at once.

## The design for Kavach

**You write ordinary Kavach.** A function marked `gpu fn` may run on the GPU. Mapping a list with it, or with a lambda that calls it, runs on the GPU when the list is long enough:

```kavach
gpu fn brightness(r: Float32, g: Float32, b: Float32, gain: Float32) -> Float32 {
    (0.2126 * r + 0.7152 * g + 0.0722 * b) * gain
}

const lit = pixels.map(p => brightness(p, p, p, 1.5))
```

How each pain is handled:

| Pain | In Kavach |
|---|---|
| Moving memory | The compiler and runtime upload the list, read the result back and free the buffers. Nothing to write |
| Launch boilerplate | None: `map` is the launch |
| Indices and bounds | There are none in the code: the item is a parameter. Out-of-bounds access can't be written |
| Races | A `gpu fn` can't write shared state. Each item's result depends only on that item and the captured numbers |
| Debugging | The same code runs on the CPU: with `KAVACH_GPU=off` the debugger, `print` and the interpreter all work. `KAVACH_GPU=check` runs both and reports the first item whose results differ |
| Two languages | One language. The compiler translates `gpu fn` to WGSL |
| Vendor lock-in | `wgpu` runs on Vulkan, DirectX 12 and Metal: NVIDIA, AMD, Intel and Apple ([decision 0044](../decisions/0044-gpu-through-wgpu.md)) |
| Silent arithmetic | Integer overflow and division by zero are checked on the GPU and panic with the same message as on the CPU |
| Tuning | The runtime picks the workgroup size and decides CPU or GPU from the list's length and the work per item, which the compiler estimates (a loop counted once per round); while the GPU opens, light work doesn't wait for it. Pipelines are compiled once and cached |
| Compile stutter | Kernels are translated while compiling the program; the GPU compiles each one at its first use, then keeps it |

**The checker makes a `gpu fn` GPU-safe.** It takes only `Float32`, `Int32`, `UInt32` and `Bool`, uses no lists, text, maps or capabilities, and calls only other `gpu fn`s and the math that WGSL has. A `gpu fn` that breaks a rule is a compile error that names the rule, so nothing fails later on the GPU.

**Results are the same as on the CPU**, item for item, with one stated exception: a GPU may fuse a multiply and an add into one step, which rounds once instead of twice. That changes the last bit of some `Float32` results, so `KAVACH_GPU=check` compares floats with `near` (14.2).

A list of `Float32`, `Int32` or `UInt32` keeps 4 bytes per item ([decision 0045](../decisions/0045-packed-lists-of-small-numbers.md)), the layout of a GPU buffer. So `map` uploads the list's own memory, and the GPU's results become the new list's memory, with no conversion either way.

**Lists stay in GPU memory for a while.** The runtime keeps the last few lists a `map` read or made (4 lists, 512 MB at most), with their GPU buffers, so a later `map` of one skips the copy over. It holds a reference to each, and Kavach's lists are copied when changed while held twice, so a kept list never changes under its GPU copy. A program sees no difference, except that changing such a list copies it once.

**A frame writes over the last.** In `ps = ps.map(step)`, when the list is on the GPU and nothing but the `map` holds it (its reference count says so: the call, the `map`'s own copy, and the runtime's), the kernel writes its results over the items, in the same buffer: each item's fields are all read before its results are written, at the same words. Its items are then bound writable, with a second bind group layout. Otherwise the results go into a buffer no list holds any more (`runtime/gpu/src/spare.rs`), or a new one, which the GPU fills with zeros first. With both, 300 frames of 1 million particles take about 70 ms after the first, against 190–200 ms with a new buffer each frame ([benchmarks/gpu](../../benchmarks/gpu/README.md#frames-records-that-stay-on-the-gpu)).

## What comes next

- Results that stay on the GPU until the CPU reads them, so chains of operations pay for the copy back once. This is where memory-bound work can win. It stays invisible, with no `GpuList` type (Mithlesh's decision, 2026-11-03).
- `sum`, `min`, `max` and `filter` on the GPU, as with `map`.
