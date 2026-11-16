# tests/conformance/15-gpu/

Conformance tests for spec chapter [15. GPU](../../../docs/spec/15-gpu.md). Format: [../README.md](../README.md). These lists are short, so they run on the CPU; `tools/cli/tests/textures.rs` samples textures, and `tools/cli/tests/gpu.rs` runs `map`s, `filter`, `sum`, `min` and `max` on the GPU (`KAVACH_GPU=on` and `check`).

| Section | Test | Rule | Result |
|---|---|---|---|
| 15.1, 15.2 | [gpu-fn-maps-a-list.kav](gpu-fn-maps-a-list.kav) | A `gpu fn`, or a lambda calling one, maps a list; literals in it are 32-bit. | runs |
| 15.1 | [gpu-fn-takes-32-bit-types.kav](gpu-fn-takes-32-bit-types.kav) | A `gpu fn` takes and gives only `Float32`, `Int32`, `UInt32` and `Bool`, and records and tuples of them. | compile error |
| 15.1 | [gpu-fn-takes-and-gives-records.kav](gpu-fn-takes-and-gives-records.kav) | A `gpu fn` takes and gives records and tuples of the four types, reading their fields and making new ones. | runs |
| 15.1 | [gpu-fn-calls-only-gpu-fns.kav](gpu-fn-calls-only-gpu-fns.kav) | A `gpu fn` calls only other `gpu fn`s, conversions and `abs`, `min`, `max`, `clamp`. | compile error |
| 15.1 | [gpu-fn-has-no-recursion.kav](gpu-fn-has-no-recursion.kav) | A `gpu fn` can't reach itself through calls. | compile error |
| 15.2 | [gpu-overflow-panics.kav](gpu-overflow-panics.kav) | Whole-number overflow in a `gpu fn` panics as on the CPU. | panic |
| 15.2 | [gpu-maps-records-and-tuples.kav](gpu-maps-records-and-tuples.kav) | A lambda calling `gpu fn`s maps a list of records or tuples of the four types, reading their fields. | runs |
| 15.1, 14.5 | [gpu-fn-uses-math.kav](gpu-fn-uses-math.kav) | A `gpu fn` may use `math.sqrt`, `floor`, `ceil`, `round` and `pow` on `Float32`. | runs |
| 15.1 | [gpu-fn-has-no-inout.kav](gpu-fn-has-no-inout.kav) | A `gpu fn` gives its result back; it can't take `inout` parameters. | compile error |
| 15.1 | [gpu-fn-uses-no-capabilities.kav](gpu-fn-uses-no-capabilities.kav) | A `gpu fn` can't use capabilities: reading a file in one is a compile error that says so. | compile error |
| 15.2 | [gpu-sum-min-max-of-whole-numbers.kav](gpu-sum-min-max-of-whole-numbers.kav) | `sum`, `min` and `max` of `Int32` or `UInt32` give the CPU's answers, and a sum that overflows panics as the CPU adds it. | panic |
| 15.2 | [gpu-sums-in-a-loop-run-in-order.kav](gpu-sums-in-a-loop-run-in-order.kav) | A loop's sums the loop doesn't wait for are added in order, each with its checks, before any code after the loop. | panic |
| 15.2 | [gpu-filter-keeps-the-order.kav](gpu-filter-keeps-the-order.kav) | `filter` of a list a GPU `map` made keeps the items the CPU keeps, in their order. | runs |
| 15.2 | [gpu-chained-maps-run-as-one.kav](gpu-chained-maps-run-as-one.kav) | `map`s chained in one expression give the results of each `map` in turn, on the GPU as on the CPU, with each lambda's captured values. | runs |
| 15.2 | [gpu-map-reading-a-captured-field-warns.kav](gpu-map-reading-a-captured-field-warns.kav) | A GPU `map` reading a field of a captured record runs on the CPU, with a warning naming the field. | runs |
| 15.3 | [gpu-vertex-and-fragment-fns-run-on-the-cpu.kav](gpu-vertex-and-fragment-fns-run-on-the-cpu.kav) | A gpu vertex fn and a gpu fragment fn are ordinary functions on the CPU too. | runs |
| 15.3 | [gpu-vertex-fn-gives-a-position.kav](gpu-vertex-fn-gives-a-position.kav) | A gpu vertex fn gives a record with the field `position`, four `Float32`s. | compile error |
| 15.3 | [gpu-fragment-fn-takes-what-the-vertex-fn-gives.kav](gpu-fragment-fn-takes-what-the-vertex-fn-gives.kav) | `gpu.pipeline`'s fragment fn takes what its vertex fn gives. | compile error |
| 15.3 | [frame-draw-gives-what-the-vertex-fn-takes.kav](frame-draw-gives-what-the-vertex-fn-takes.kav) | `frame.draw` gives the instances and shared value its pipeline's vertex fn takes: a second parameter is the instance. | compile error |
| 15.3 | [gpu-pipeline-takes-gpu-fns.kav](gpu-pipeline-takes-gpu-fns.kav) | `gpu.pipeline` takes a gpu vertex fn and a gpu fragment fn, by name. | compile error |
| 15.3 | [gpu-pipeline-depth-is-true-or-false.kav](gpu-pipeline-depth-is-true-or-false.kav) | `depth:` of `gpu.pipeline` is written `true` or `false`: it is part of the pipeline, compiled before it draws. | compile error |
| 15.3 | [gpu-fragment-fn-takes-textures-after-its-input.kav](gpu-fragment-fn-takes-textures-after-its-input.kav) | After what the vertex fn gives, a gpu fragment fn takes only `Texture`s. | compile error |
| 15.3 | [frame-draw-gives-the-textures-its-fragment-fn-reads.kav](frame-draw-gives-the-textures-its-fragment-fn-reads.kav) | `frame.draw` gives the textures its pipeline's fragment fn takes, as `textures:`. | compile error |
| 15.3 | [gpu-fn-reads-a-texture-only-by-sample.kav](gpu-fn-reads-a-texture-only-by-sample.kav) | `sample` is the one call on a `Texture` a gpu fn can make. | compile error |
| 15.3, 14.28 | [gpu-fn-uses-std-vec.kav](gpu-fn-uses-std-vec.kav) | A gpu fn may use std.vec's types of Float32, their operators and methods; a vertex fn's position and a fragment fn's color may be a Vec4. | runs |
