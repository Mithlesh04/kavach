# compiler/backend/wgsl/

KIR to WGSL: the kernel of each `map` that may run on the GPU (spec [chapter 15](../../../docs/spec/15-gpu.md), [decision 0044](../../../docs/decisions/0044-gpu-through-wgpu.md)). The CLI calls `attach` after lowering; the GPU runtime (`runtime/gpu`) compiles each kernel at its first use.

| File | Job |
|---|---|
| [src/lib.rs](src/lib.rs) | The kernel frame (buffers, checked arithmetic, the stop flag), `kernel` and `attach` |
| [src/expr.rs](src/expr.rs) | KIR function bodies as WGSL functions |
| [src/ops.rs](src/ops.rs) | Operators as WGSL, and whether written code can stop an item |
| [src/cost.rs](src/cost.rs) | The work per item, roughly, which the runtime weighs against starting the GPU |
| [src/filter.rs](src/filter.rs) | The kernel of `filter`: run once to count each workgroup's kept items, then to write them in order |
| [src/reduce.rs](src/reduce.rs) | Kernels for `sum`, `min` and `max` of 32-bit whole numbers: one ordered partial per workgroup |
| [src/tests.rs](src/tests.rs) | Kernels written from whole programs: the work estimate, record items |
