# compiler/backend/

Code generators: KIR → machine code ([decision 0005](../../docs/decisions/0005-backends-cranelift-llvm-wasm.md)).

| Folder | Backend | Status |
|---|---|---|
| [cranelift/](cranelift/README.md) | Cranelift: fast compiles for development | Runs the KIR subset in memory (`kavach run --native`) |
| [llvm/](llvm/README.md) | LLVM: optimized release builds ([decision 0018](../../docs/decisions/0018-llvm-backend-through-llvm-tools.md)) | `kavach build` uses it for the part of KIR it supports so far |
| [wgsl/](wgsl/README.md) | WGSL: the kernels of `gpu fn`s, run on the GPU through `wgpu` ([decision 0044](../../docs/decisions/0044-gpu-through-wgpu.md)) | `map` with a `gpu fn` on `Float32`, `Int32`, `UInt32`, `Bool` |
