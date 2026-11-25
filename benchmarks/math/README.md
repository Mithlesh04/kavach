# Math benchmarks

Arithmetic over large data (to-do item 1: the math target). Each program is small code on a big list, in Kavach and in C++ (`g++ -O2`), timed whole:

| Benchmark | What it measures |
|---|---|
| `sum` | Summing 50 million integers, 10 times (overflow checked in Kavach, not in C++) |
| `dot` | The dot product of two 20-million-float lists, 10 times; the sum's order is fixed, so neither language may widen it |
| `scale` | Element-wise `y[i] = x[i] * 2.5 + 1.0` over 20 million floats, 10 times: every element stands alone |
| `roots` | `sqrt` over 20 million floats, 10 times; exactly rounded, so doing several at once is allowed |
| `matmul` | The product of two 1024 × 1024 float matrices, 4 times, as `Tensor`s (spec 14.33): each item adds its products in order, the C++ loop's order, so both give the same bits. Kavach's product uses all cores; `cpp/matmul_cores.cpp` is the C++ loop on all cores (`g++ -O2 -fopenmp`). Also in NumPy (`numpy/`) and Eigen (`eigen/`), below |

Run them (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/math
```

## Results

Measured 2026-11-09 at commit b65b7ae7 (`kavach --version`; WSL Ubuntu, g++ 13, i7-13700HX), in three rounds of best of 9 one after another; each row is the best of the three rounds, and the ratio in each round is under the table. Ratio is Kavach ÷ C++, so below 1 is faster than C++.

**All on one core.** These four programs run on one core in both languages: none has a loop written `on cores` ([spec 4.9](../../docs/spec/04-control-flow.md#49-where-a-loop-runs)). On 2026-11-09, before [decision 0126](../../docs/decisions/0126-loops-on-one-core-unless-on-cores.md), Kavach spread `sum`, `scale` and `roots` over all cores by itself, against C++ on one core: 135, 110 and 201 ms. Those times were not like for like, so their rows wait for a new run (*to measure*).

| Benchmark | Kavach ms | C++ ms | Ratio | |
|---|---|---|---|---|
| `sum` | to measure | 363 | to measure | one core each, every Kavach addition still overflow-checked |
| `scale` | to measure | 274 | to measure | one core each, element-wise |
| `roots` | to measure | 433 | to measure | one core each |
| `dot` | 184 | 198 | **0.93** | 1.1× faster; a float sum keeps its exact adding order, so it was on one core then too |

The ratio in each round: `dot` 0.86, 0.90, 0.93. On 2026-11-02 both sides took longer (C++'s `sum` 601 ms), so compare ratios measured the same day.

`matmul` on all cores, measured 2026-11-15 at commit 4ef54c72 (the same machine, best of 9): Kavach 261 ms, C++ to measure (`cpp/matmul_cores.cpp`, the i-k-j loop with its rows spread over the cores by OpenMP), the same bits. Kavach's exact product spreads rows over the cores and keeps a block of the second matrix in each core's cache.

`matmul` on one core: Kavach to measure (run with `KAVACH_CORES=1`), C++ 1315 ms (2026-11-15), the plain i-k-j loop, the same bits.

Until 2026-11-25 this page compared Kavach's 261 ms on all cores with C++'s 1315 ms on one core and called it 5× faster; that was not like for like. `fast: true` (through `gemm`), NumPy (OpenBLAS) and Eigen are compared [below](#products-against-numpy-and-eigen).

The target is **20× as fast as C++** (0.05), from SIMD and all cores; reaching it on all cores means 20× a C++ program on all cores too. Both steps of [decision 0031](../../docs/decisions/0031-loops-on-all-cores.md) landed 2026-11-02: an eligible loop is outlined into a chunk function and fanned out over a kept worker pool, with the plain loop as the serial path — a chunk whose check fails, or a loop too small to pay for threads, re-runs plainly, so results and panics stay exactly the plain loop's (four conformance tests hold it, including an overflow only a serial prefix sees). Since [decision 0126](../../docs/decisions/0126-loops-on-one-core-unless-on-cores.md) (2026-11-17), only a loop written `on cores` is spread. Floats are never re-ordered, so `dot` waits for an asked-for reordered sum; the rest of the way to 20× is SIMD width. Two steps so far (2026-11-01):

1. **One instruction per operation.** `list[i] = v` and `math.sqrt` each called the runtime: `scale` was 4034 ms (12× *slower* than C++), `roots` 4695 ms.
2. **No checks inside a proven loop.** For `for i in 0..n { ys[i] = … }`, one check before the loop (the range fits, and `ys` is unshared) covers every round, so the fast copy stores straight through the buffer and LLVM may use vector instructions; when the check fails, the loop runs as written, checking each round. `roots` went 20× faster in all: 4695 → 229 ms.

What remains for 20×: reordered sums where the program asks for them, and splitting such loops across cores. Tried and measured (2026-11-01): tagging only float-store functions with AVX2 did make `roots` 4 square roots wide (`llvm.sqrt.v4f64`), but no faster than the baseline's 2-wide — the loop is bound by memory as much as by `sqrt` — so it stays off.

## Products against NumPy and Eigen

Measured 2026-11-23 on the machine of [benchmarks/README.md](../README.md) (WSL Ubuntu 24.04, kernel 5.15.167.4), at Kavach commit b680f1b58, when `Matrix` gave way to `Tensor` (decision 0136). Each figure is the best of 5 runs of the whole program, timed with `date +%s%N`, the programs taking turns; two such rounds, one after the other. Other work on the machine kept its load average near 5.

| Program | Round 1 ms | Round 2 ms | Result | |
|---|---|---|---|---|
| Kavach `a @ b` on tensors | 242 | 235 | 742.0316000000003 | The same bits as C++; rows spread over the cores |
| Kavach `Matrix` `a @ b`, before (commit 81e9b20f5) | 247 | 229 | 742.0316000000003 | The speed `Tensor` keeps |
| Kavach `Tensor` `a @ b`, before (through `Matrix`) | 274 | 276 | 742.0316000000003 | A copy of each operand is gone now |
| Kavach `a.matmul(b, fast: true)` (`gemm`) | 168 | 156 | 742.0316 | Another order of sums: other last bits; rows spread over the cores |
| C++ i-k-j loop, `g++ -O2`, one core | 987 | 1040 | 742.03160000000025 | Kavach's bits, on one core: compare it with Kavach on one core, not with the rows above |
| C++ i-k-j loop, `g++ -O2 -fopenmp`, rows on all cores | to measure | to measure | 742.03160000000025 | Kavach's bits, on all cores like the rows above |
| Eigen 3.4.0, `a * b`, `g++ -O2 -march=native -fopenmp` | 126 | 135 | 742.03159999999991 | Its own blocked, vectorized product on all cores |
| NumPy 1.26.4 with OpenBLAS 0.3.26 (pthreads), `python3` | 644 | 584 | 742.0315999999999 | Of which `import numpy` alone is 316 ms; the four products alone take 101 ms |

- **No slowdown from the move to tensors:** `a @ b` on tensors runs as fast as `Matrix`'s did, and faster than the old tensor path, which copied both operands into a `Matrix` first.
- **The exact product against the fast ones.** Kavach's default product gives the bits of the plain loop, on every machine. Eigen and OpenBLAS add in other orders and fuse multiplies with adds, so their last bits differ, and they are about 1.8× as fast on this machine. `fast: true` closes most of that gap: 1.2× Eigen's time.
- NumPy is Ubuntu's `python3-numpy`, which uses the reference BLAS. For this table it loaded Ubuntu's `libopenblas0-pthread`, unpacked into a build folder and put first with `LD_LIBRARY_PATH`, so nothing on the system changed; the loaded libraries were checked in `/proc/self/maps`.
- **Cores.** Every Kavach row, Eigen and OpenBLAS use all cores. The C++ one-core row is there for its bits, not to compare speed with them.
- Run them: build `eigen/matmul.cpp` with the flags above, and run `numpy/matmul.py` with `python3`; the Kavach and C++ programs, on one core and on all, run with `sh scripts/run-benchmarks.sh "" benchmarks/math`.
