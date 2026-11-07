# Math benchmarks

Arithmetic over large data (to-do item 1: the math target). Each program is small code on a big list, in Kavach and in C++ (`g++ -O2`), timed whole:

| Benchmark | What it measures |
|---|---|
| `sum` | Summing 50 million integers, 10 times (overflow checked in Kavach, not in C++) |
| `dot` | The dot product of two 20-million-float lists, 10 times; the sum's order is fixed, so neither language may widen it |
| `scale` | Element-wise `y[i] = x[i] * 2.5 + 1.0` over 20 million floats, 10 times: every element stands alone |
| `roots` | `sqrt` over 20 million floats, 10 times; exactly rounded, so doing several at once is allowed |

Run them (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/math
```

## Results

Measured 2026-11-02 (WSL Ubuntu, g++ 13, i7-13700HX), best of 9; ratio is Kavach ÷ C++, so below 1 is faster than C++.

| Benchmark | Kavach ms | C++ ms | Ratio | |
|---|---|---|---|---|
| `sum` | 208 | 601 | **0.35** | 2.9× faster than C++, every addition still overflow-checked: integer sums run on all cores and combine exactly |
| `scale` | 173 | 467 | **0.37** | 2.7× faster than C++: element-wise on all cores; memory bandwidth is the wall |
| `roots` | 279 | 567 | **0.49** | 2.0× faster than C++ |
| `dot` | 266 | 339 | **0.78** | 1.3× faster; a float sum keeps its exact adding order (decision 0031), so it stays on one core |

The target is **20× as fast as C++** (0.05), from SIMD and all cores. Both steps of [decision 0031](../../docs/decisions/0031-loops-on-all-cores.md) landed 2026-11-02: an eligible loop is outlined into a chunk function and fanned out over a kept worker pool, with the plain loop as the serial path — a chunk whose check fails, or a loop too small to pay for threads, re-runs plainly, so results and panics stay exactly the plain loop's (four conformance tests hold it, including an overflow only a serial prefix sees). Floats are never re-ordered, so `dot` waits for an asked-for reordered sum; the rest of the way to 20× is SIMD width. Two steps so far (2026-11-01):

1. **One instruction per operation.** `list[i] = v` and `math.sqrt` each called the runtime: `scale` was 4034 ms (12× *slower* than C++), `roots` 4695 ms.
2. **No checks inside a proven loop.** For `for i in 0..n { ys[i] = … }`, one check before the loop (the range fits, and `ys` is unshared) covers every round, so the fast copy stores straight through the buffer and LLVM may use vector instructions; when the check fails, the loop runs as written, checking each round. `roots` went 20× faster in all: 4695 → 229 ms.

What remains for 20×: reordered sums where the program asks for them, and splitting such loops across cores. Tried and measured (2026-11-01): tagging only float-store functions with AVX2 did make `roots` 4 square roots wide (`llvm.sqrt.v4f64`), but no faster than the baseline's 2-wide — the loop is bound by memory as much as by `sqrt` — so it stays off.
