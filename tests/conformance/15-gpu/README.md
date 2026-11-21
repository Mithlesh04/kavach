# tests/conformance/15-gpu/

Conformance tests for spec chapter [15. GPU](../../../docs/spec/15-gpu.md). Format: [../README.md](../README.md). These lists are short, so they run on the CPU; `tools/cli/tests/textures.rs` samples textures, `tools/cli/tests/gpu.rs` runs `map`s, `filter`, `sum`, `min` and `max` on the GPU (`KAVACH_GPU=on` and `check`), `tools/cli/tests/gpu_choice.rs` chooses among GPUs listed by `KAVACH_FAKE_GPUS` and real ones (15.6), and `tools/cli/tests/gpu_passes.rs` runs a `map` in several passes of the GPU.

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
| 14.24, 8.1 | [gpu-screen-needs-window.kav](gpu-screen-needs-window.kav) | Drawing changes the program's own windows, so a `pub` function that calls `gpu.screen` must say `uses Window`. | compile error |
| 14.24 | [gpu-texture-pixels-are-four-per-pixel.kav](gpu-texture-pixels-are-four-per-pixel.kav) | `gpu.texture`'s pixels are four for each pixel: another length than `width * height * 4` is a panic. | panic |
| 15.1 | [gpu-fn-calls-no-plain-fn.kav](gpu-fn-calls-no-plain-fn.kav) | A `gpu fn` calls only other `gpu fn`s: calling a plain `fn` of the same types is a compile error. | compile error |
| 15.1, 2.1 | [gpu-fn-converts-and-clamps.kav](gpu-fn-converts-and-clamps.kav) | A `gpu fn` may convert between `Float32`, `Int32` and `UInt32`, and use `abs`, `min`, `max` and `clamp`. | runs |
| 15.1 | [gpu-fn-gives-only-32-bit-types.kav](gpu-fn-gives-only-32-bit-types.kav) | A `gpu fn` gives only `Float32`, `Int32`, `UInt32` and `Bool`, or records and tuples of them. | compile error |
| 15.1 | [gpu-fn-has-no-error-type.kav](gpu-fn-has-no-error-type.kav) | A `gpu fn` can't fail: `!` in its signature is a compile error. | compile error |
| 15.1 | [gpu-fn-has-no-match.kav](gpu-fn-has-no-match.kav) | A `gpu fn`'s body uses only the statements 15.1 lists (`if` and `else`, `while`, `for` over a range, `return`, …): `match` is not one. | compile error |
| 15.1 | [gpu-fn-locals-are-32-bit.kav](gpu-fn-locals-are-32-bit.kav) | A `gpu fn`'s `const` and `let` hold only the 32-bit types and `Bool`. | compile error |
| 15.1 | [gpu-fn-makes-no-lists.kav](gpu-fn-makes-no-lists.kav) | A `gpu fn` uses no lists, in its body as in its signature. | compile error |
| 15.1 | [gpu-fn-power-is-float32-only.kav](gpu-fn-power-is-float32-only.kav) | In a `gpu fn`, `**` works on `Float32` only; on whole numbers it is a compile error. | compile error |
| 15.1 | [gpu-fn-record-fields-are-32-bit.kav](gpu-fn-record-fields-are-32-bit.kav) | The records a `gpu fn` takes have fields only of `Float32`, `Int32`, `UInt32` and `Bool`. | compile error |
| 15.1 | [gpu-fn-records-have-at-most-8-fields.kav](gpu-fn-records-have-at-most-8-fields.kav) | A record a `gpu fn` takes or gives has 1 to 8 fields. | compile error |
| 15.1, 14.5 | [gpu-fn-uses-angles-exp-and-ln.kav](gpu-fn-uses-angles-exp-and-ln.kav) | A `gpu fn` may use `math.sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp` and `ln` on `Float32`; they give a `Float32`. | runs |
| 15.1 | [gpu-fn-uses-no-maps.kav](gpu-fn-uses-no-maps.kav) | A `gpu fn` uses no maps. | compile error |
| 15.1 | [gpu-fn-uses-no-text.kav](gpu-fn-uses-no-text.kav) | A `gpu fn` uses no text. | compile error |
| 15.1 | [gpu-fn-uses-while-if-and-return.kav](gpu-fn-uses-while-if-and-return.kav) | A `gpu fn` may use `let`, assignment, `if` and `else`, `while`, `for` over a range and `return`. | runs |
| 15.1, 15.2 | [gpu-results-match-direct-calls.kav](gpu-results-match-direct-calls.kav) | A `map` gives what calling its `gpu fn` on each item directly gives: the code and the results are the same wherever it runs. | runs |
| 15.2 | [gpu-chained-map-panic-gives-its-message.kav](gpu-chained-map-panic-gives-its-message.kav) | A panic in the second of two chained `map`s stops the work with that map's message, as on the CPU. | panic |
| 15.2, 5.6 | [gpu-division-by-zero-panics.kav](gpu-division-by-zero-panics.kav) | Whole-number division by zero in a `gpu fn` that a `map` runs panics, as it does on the CPU. | panic |
| 15.2, 2.2 | [gpu-mapped-list-changes-by-copy.kav](gpu-mapped-list-changes-by-copy.kav) | A list a GPU `map` made is an ordinary list: changing a second holder of it copies it first. | runs |
| 15.2 | [gpu-maps-in-a-loop-are-read-after-it.kav](gpu-maps-in-a-loop-are-read-after-it.kav) | A loop of `map`s feeding each result to the next gives, at the first read after the loop, every item as the CPU works it out. | runs |
| 15.3, 14.24 | [frame-draw-vertices-are-what-the-vertex-fn-takes.kav](frame-draw-vertices-are-what-the-vertex-fn-takes.kav) | `frame.draw`'s vertices are a list of the record its pipeline's vertex fn takes first. | compile error |
| 15.3 | [gpu-fragment-fn-gives-a-color.kav](gpu-fragment-fn-gives-a-color.kav) | A `gpu fragment fn` gives a color: four `Float32`s, or a `Vec4<Float32>`. | compile error |
| 15.3 | [gpu-only-fragment-fns-take-textures.kav](gpu-only-fragment-fns-take-textures.kav) | Only a `gpu fragment fn` takes textures: a `gpu vertex fn` taking a `Texture` is a compile error. | compile error |
| 15.3, 1.6 | [gpu-vertex-and-fragment-are-names-elsewhere.kav](gpu-vertex-and-fragment-are-names-elsewhere.kav) | `vertex` and `fragment` are words only between `gpu` and `fn`; anywhere else they are ordinary names. | runs |
| 15.3 | [gpu-vertex-fields-are-numbers.kav](gpu-vertex-fields-are-numbers.kav) | A vertex's fields are `Float32`s, `Int32`s or `UInt32`s: a `Bool` field is a compile error. | compile error |
| 15.3, 15.1 | [gpu-vertex-fn-follows-gpu-fn-rules.kav](gpu-vertex-fn-follows-gpu-fn-rules.kav) | A `gpu vertex fn` follows the rules of a `gpu fn`: it can't print. | compile error |
| 15.3 | [gpu-vertex-fn-gives-float32-fields.kav](gpu-vertex-fn-gives-float32-fields.kav) | Besides `position`, the fields a `gpu vertex fn` gives to the fragment fn are `Float32`s. | compile error |
| 15.3, 5.6 | [gpu-vertex-fn-panics-on-the-cpu.kav](gpu-vertex-fn-panics-on-the-cpu.kav) | A `gpu vertex fn` called on the CPU panics on whole-number overflow, as any other function does. | panic |
| 15.3 | [gpu-vertex-fn-takes-at-most-three-records.kav](gpu-vertex-fn-takes-at-most-three-records.kav) | A `gpu vertex fn` takes one, two or three records: the vertex, the instance and the shared value. | compile error |
| 15.3 | [gpu-vertex-fn-takes-records.kav](gpu-vertex-fn-takes-records.kav) | What a `gpu vertex fn` takes are records, not bare numbers. | compile error |
| 15.6, 8.1 | [gpu-choosing-needs-uses-gpu.kav](gpu-choosing-needs-uses-gpu.kav) | Choosing a GPU needs `uses Gpu`: a `pub` function with `on gpu(n)` must say so. | compile error |
| 15.6, 14.24, 8.1 | [gpu-devices-needs-uses-gpu.kav](gpu-devices-needs-uses-gpu.kav) | Listing the GPUs with `gpu.devices()` needs `uses Gpu`. | compile error |
| 15.6 | [gpu-map-needs-no-capability.kav](gpu-map-needs-no-capability.kav) | A `gpu fn` and its `map` need no capability when the code chooses no GPU. | runs |
| 15.6, 4.9 | [gpu-numbers-start-at-zero.kav](gpu-numbers-start-at-zero.kav) | GPUs are numbered from 0: a negative number written in `on gpu(n)` is a compile error. | compile error |
| 15.6 | [gpu-n-fn-takes-a-number.kav](gpu-n-fn-takes-a-number.kav) | `gpu(n) fn` takes a device's number written in the code. | compile error |
| 15.6 | [gpu-map-on-takes-a-device.kav](gpu-map-on-takes-a-device.kav) | `on:` on a `map` takes a `GpuDevice` from `gpu.devices()`, not a number. | compile error |
| 15.6 | [gpu-one-map-runs-on-one-gpu.kav](gpu-one-map-runs-on-one-gpu.kav) | A lambda given to `map` can't call `gpu(n) fn`s of two numbers. | compile error |
| 15.6, 14.24 | [gpu-on-gives-the-same-values.kav](gpu-on-gives-the-same-values.kav) | A `map` gives the same values on every device `gpu.on` or `on:` chooses. | runs |
| 15.6 | [gpu-missing-device-stops-at-start.kav](gpu-missing-device-stops-at-start.kav) | A `gpu(n) fn` naming a GPU the machine doesn't have stops the program at its start. | exit 1 |
| 15.6 | [gpu-choosing-a-missing-device-panics.kav](gpu-choosing-a-missing-device-panics.kav) | A GPU number worked out while running that names no device is a panic. | panic |
