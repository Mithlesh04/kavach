# compiler/backend/wgsl/

KIR to WGSL: the kernel of each `map` that may run on the GPU (spec [chapter 15](../../../docs/spec/15-gpu.md), [decision 0044](../../../docs/decisions/0044-gpu-through-wgpu.md)). The CLI calls `attach` after lowering; the GPU runtime (`runtime/gpu`) compiles each kernel at its first use.

| File | Job |
|---|---|
| [src/lib.rs](src/lib.rs) | The kernel frame (buffers, checked arithmetic, the stop flag), `kernel` and `attach` |
| [src/expr.rs](src/expr.rs) | KIR function bodies as WGSL functions |
| [src/render.rs](src/render.rs) | A pipeline's shaders: the vertex and fragment fns with `vs_main` and `fs_main` around them, and the buffers' layout, depth and texture count first (spec 15.3) |
| [src/texture.rs](src/texture.rs) | A fragment fn's `Texture` parameters as `texture_2d<f32>`, and `sample` through the pipeline's sampler (spec 15.3) |
| [src/convert.rs](src/convert.rs) | Conversions, checked as the CPU checks them, and the `std.math` calls a `Float32` power or root arrives as |
| [src/ops.rs](src/ops.rs) | Operators as WGSL, and whether written code can stop an item |
| [src/cost.rs](src/cost.rs) | The work per item, roughly, which the runtime weighs against starting the GPU |
| [src/filter.rs](src/filter.rs) | The kernel of `filter`: run once to count each workgroup's kept items, then to write them in order |
| [src/reduce.rs](src/reduce.rs) | Kernels for `sum`, `min` and `max` of 32-bit whole numbers: one ordered partial per workgroup |
| [src/tests.rs](src/tests.rs) | Kernels written from whole programs: the work estimate, record items |
| [src/tests_draw.rs](src/tests_draw.rs) | Depth and textures, written and drawn off-screen (spec 15.3) |
