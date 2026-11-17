# 0125. Kavach for AI work: checked first, fast where it counts

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

A maintainer asked for frameworks on Kavach for AI infrastructure engineers, scientists and model trainers, and for the capabilities many such programs share to be Kavach's own (to-do 19). A research session compared Python, PyTorch, JAX and Mojo with what goes wrong in AI work. The costly failures are not speed: they come late, stay silent or can't be repeated. Shapes that don't fit, low-precision numbers that lose meaning without a word, runs that differ between machines, a cluster hung by one machine taking another branch, lost progress after a crash, model files that run code, and AI-written kernels that pass weak tests. Kavach already has what answers these: the same bits everywhere (decision 0122), contracts and generated tests (chapter 7, decision 0010), durable runs (decision 0025), guarded agents (chapter 8) and one native binary. So Kavach aims to be the checked layer, and calls other engines (MAX, vLLM, llama.cpp) for peak inference, rather than compete on peak GPU kernels or big distributed training yet.

A maintainer chose the research's recommendation for each of the eight questions below, on 2026-11-16.

## Decision

- **D1 Shapes are checked at run time first.** `Tensor<T>` (`std.tensor`) checks every operation's shapes as it runs, with errors naming both shapes. Shapes in the type (`Tensor<Float32, [B, 768]>`) come later, once generics take values (to-do 14).
- **D2 Low precision is storage.** `BFloat16` and `Float16` hold numbers; arithmetic on them works in `Float32` and says so. fp8 and fp4 exist only inside tensors, in blocks with their scales.
- **D3 GPU tensors give the same bits by default.** Every operation gives the same result on every machine and at every thread count. `fast: true`, or a `fast` block, opts into fused multiply-adds and any order of sums.
- **D4 Autodiff is a library tape first**, in `std.tensor`, with generated gradient checks against finite differences. A compiler transform may come later.
- **D5 The GPU goes through wgpu now** (any vendor), as Kavach's GPU code does today (chapter 15). Vendor libraries (cuBLAS, rocBLAS) come later, behind `fast:`. Kavach writes no PTX backend of its own.
- **D6 Kernel contracts are a language clause**: a kernel names a reference function and an error bound, and Kavach generates adversarial tests and a benchmark from them.
- **D7 Durable jobs reach beyond agents**, in the language: a training run resumes after a crash from its last checkpoint, as an agent replays its journal (decision 0025).
- **D8 Collectives come later**, with a compile error on a branch that depends on the machine's rank and a check at run time that every machine agrees.
- **Tensors are written in Kavach**, in `std.tensor`, on lists and the operator interfaces (2.16), building on `Matrix<T>` (2.17, decision 0111) for the products. A compiler or runtime change is made only where speed needs it, with its own decision.

## Consequences

- AI code in Kavach fails early and loudly, and repeats: the same model gives the same logits on Windows and Linux at every thread count. The first goal is GPT-2 small from safetensors as one native binary that does that.
- Packages in their own repositories carry what Kavach itself needn't: safetensors (then GGUF and ONNX), run records, data pipelines, nn, models, RL and serving on Rath.
- The same-bits default costs speed against frameworks that don't keep it; `fast` is there when a result needn't repeat.

## Alternatives considered

- **Shapes in the type now (D1):** caught while compiling, but it needs value parameters in generics first, so tensors would come much later.
- **Vendor libraries first (D3, D5):** the top speed, but results that differ between machines, and NVIDIA or AMD only.
- **Autodiff as a compiler transform first (D4):** faster and safer from the start, but a large compiler change before anyone has used tensors.
- **No kernel contracts (D6):** what other languages do; AI-written kernels then pass on weak tests, the failure the research found most often.
