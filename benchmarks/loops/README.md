# benchmarks/loops

Small benchmarks of the basics, in Kavach and C++, doing exactly the same work on large data. They show whether native Kavach code is faster than C++, and [against Go and C#](#against-go-and-c), than those ([to-do item 1](../../docs/vision/todo.md)): the goal is Kavach at least 2× as fast as `g++ -O2` everywhere, and 3× on plain loops.

| Benchmark | What it measures |
|---|---|
| `loop` | A tight `for` loop with integer arithmetic: 500 million rounds |
| `fib` | Function calls: recursive `fib(38)`, about 126 million calls |
| `lists` | A growing list: 20 million `push`es, then a loop that adds them up |
| `floats` | Floating-point arithmetic in a loop |
| `matrix` | Nested lists: multiplying two matrices |
| `records` | 10 million records in a list: create, then read their fields |
| `maps` | A map with 2 million integer keys: insert, then look every key up |
| `sizedmaps` | The same, with the map made with room for its 2 million keys (`Map.capacity`; C++ `reserve`s the same) |
| `text` | Building, splitting and searching large text |
| `sort` | Sorting a large list |
| `closures` | Calling function values in a loop |

## Results

Linux (WSL Ubuntu 24.04), Intel i7-13700HX. Kavach: `kavach build` (LLVM backend). C++: g++ 13.3 `-O2`. Best of 9 runs (Kavach and C++ alternating), in milliseconds. **Ratio** is Kavach's time divided by C++'s: below 1 means Kavach is faster (0.50 = twice as fast). Kavach checks every integer operation for overflow and every list index; C++ doesn't. Timings vary by about 10% between runs.

Measured 2026-11-09 at commit b65b7ae7 (`kavach --version`), in three rounds of best of 9 one after another; each row is the best of the three rounds. On 2026-11-02 the same programs gave other times on both sides (C++'s `lists` took 189 ms then, 74–83 now), so compare ratios measured the same day.

**One core or all cores.** Every row compares programs on the same number of cores. Three Kavach programs use all cores: `loop` and `lists` have a loop written `on cores` ([spec 4.9](../../docs/spec/04-control-flow.md#49-where-a-loop-runs)), and `sort` sorts 5 million numbers, which the library sorts in 256 buckets on up to 8 cores. Each of these has two rows: **1 core**, Kavach run with `KAVACH_CORES=1` against the plain C++ program, and **all cores**, Kavach as built against a C++ program on all cores (`cpp/<name>_cores.cpp`, `g++ -O2 -fopenmp`). Every other program runs on one core in every language. Until 2026-11-25 these rows compared Kavach on all cores with C++ on one core; a cell that says *to measure* waits for a new run.

| Benchmark | Kavach ms | C++ ms | Ratio | Status |
|---|---|---|---|---|
| `loop` · 1 core | to measure | 1435 | to measure | one core each; Kavach's `(total + x) % m` needs a compare and a subtract where C++ divides ([below](#progress)) |
| `loop` · all cores | 56 | to measure | to measure | both on all cores: Kavach's `on cores` loop, C++ with an OpenMP reduction of the same sums |
| `sort` · 1 core | to measure | 378 | to measure | one core each: radix sort against `std::sort` |
| `sort` · all cores | 34 | to measure | to measure | both on all cores: 256 buckets by the top digit, sorted on up to 8 cores (Kavach) and on all of them (C++) |
| `closures` | to measure | 98 | to measure | one core each: the fused `map`/`filter`/`sum` pipeline. The 20 ms measured 2026-11-09 was on all cores, before loops ran on one core unless written `on cores` ([decision 0126](../../docs/decisions/0126-loops-on-one-core-unless-on-cores.md)); C++'s time varied most (98–284 ms) |
| `records` | 44 | 115 | **0.38** | 2.6× faster than C++ |
| `lists` · 1 core | to measure | 74 | to measure | one core each: byte-packed lists (decision 0030) |
| `lists` · all cores | 30 | to measure | to measure | both on all cores: pushing stays on one core, the summing pass is spread |
| `text` | 32 | 63 | **0.51** | 2.0× faster than C++; most of the time is the kernel zeroing fresh pages (C++ pays it too) |
| `matrix` | 8 | 15 | **0.53** | 1.9× faster than C++ (times this short vary by a millisecond) |
| `sizedmaps` | 45 | 73 | **0.62** | 1.6× faster than C++ when both are told the size: no index to grow, which is most of what `maps` spends ([hash maps](../../docs/architecture/hash-maps.md#open-questions)) |
| `maps` | 64 | 88 | **0.73** | 1.4× faster than C++; was 2.0× — the one-miss buckets that made random lookups 1.5× faster than C++ ([lookups](../lookups/README.md)) cost this sequential scan a little |
| `fib` | 49 | 58 | **0.84** | 1.2× faster than C++ |
| `floats` | 372 | 362 | 1.03 | as fast as C++: each round waits for the last multiply and add, for every compiler |

The ratio in each round, in order (the rows measured one core against one core): `records` 0.39, 0.39, 0.38; `text` 0.51, 0.51, 0.52; `matrix` 0.53, 0.56, 0.53; `sizedmaps` 0.66, 0.58, 0.61; `maps` 0.73, 0.67, 0.71; `fib` 0.84, 0.84, 0.85; `floats` 1.03, 1.02, 1.02.

`kavach build` uses the LLVM backend ([decision 0018](../../docs/decisions/0018-llvm-backend-through-llvm-tools.md)) for every benchmark.

## Against Go and C#

The same eleven programs written in Go ([go/](go)) and C# ([csharp/](csharp)), line for line as the C++ ones: plain loops, the standard library's list (`append`, `List<T>`), map (`map`, `Dictionary`), text and sort. Go 1.23.4 (`go build`), C# on .NET 8.0.428 (`dotnet publish -c Release`, the default JIT). Measured 2026-11-11 with `kavach 0.1.0 (commit 445b5b61)` and this table's commit's change to freeing lists, in three rounds of best of 9 runs, all four languages in turns; each cell is the best of the three rounds, in milliseconds. Every program printed the same result in every language. An empty C# program takes 105 ms to start .NET and end (Go: 3 ms), and every C# time below includes it.

As in [Results](#results), `loop`, `sort` and `lists` have a row for one core and a row for all cores. The all-core programs are `go/<name>_cores.go` (a goroutine per core) and `csharp/<name>_cores.cs` (`Parallel.ForEach` over ranges, `Parallel.For` over the sort's buckets); each prints the one-core result. The figures from 2026-11-11 for these three compared Kavach on all cores with the others on one core, so each now stands in the row it was measured for, and the rest wait for a new run.

| Benchmark | Kavach ms | C++ ms | Go ms | C# ms | Kavach against Go | Kavach against C# |
|---|---|---|---|---|---|---|
| `loop` · 1 core | to measure | 1361 | 1377 | 1561 | to measure | to measure |
| `loop` · all cores | 60 | to measure | to measure | to measure | to measure | to measure |
| `sort` · 1 core | to measure | 361 | 450 | 572 | to measure | to measure |
| `sort` · all cores | 32 | to measure | to measure | to measure | to measure | to measure |
| `closures` | to measure | 95 | 136 | 238 | to measure | to measure |
| `maps` | 60 | 86 | 403 | 232 | **6.7× faster** | **3.9× faster** |
| `sizedmaps` | 49 | 69 | 250 | 218 | **5.1× faster** | **4.4× faster** |
| `matrix` | 8 | 14 | 37 | 135 | **4.6× faster** | **17× faster** |
| `lists` · 1 core | to measure | 73 | 124 | 211 | to measure | to measure |
| `lists` · all cores | 32 | to measure | to measure | to measure | to measure | to measure |
| `fib` | 48 | 59 | 166 | 267 | **3.5× faster** | **5.6× faster** |
| `text` | 38 | 71 | 109 | 257 | 2.9× faster | **6.8× faster** |
| `records` | 43 | 107 | 102 | 196 | 2.4× faster | **4.6× faster** |
| `floats` | 347 | 348 | 355 | 433 | about the same | 1.2× faster |

In bold: at least 3× as fast, the target for plain loops ([to-do item 1](../../docs/vision/todo.md)). `closures` is on one core in every language now; its Kavach time of 21 ms was the pipeline on all cores, before [decision 0126](../../docs/decisions/0126-loops-on-one-core-unless-on-cores.md). `floats` is as fast in every language: each round waits for the last multiply and add, whatever compiles it. Go's map and C#'s start-up cost them most; Go's `fib` pays for its growable stacks. Each time in round 1, 2 and 3 is in the run's log; round 3 ran on a busier machine, about 1.6× slower for all four languages alike.

## Progress

**40 improvements on 2026-10-31.** How much faster Kavach got on each benchmark, from its first measured ratio to 2026-11-02 (C++ didn't change). [Results](#results) has the latest ratios. These tables are a record of Kavach's own progress, not like-for-like comparisons: C++ always ran on one core, and from [decision 0031](../../docs/decisions/0031-loops-on-all-cores.md) (2026-11-02) until [decision 0126](../../docs/decisions/0126-loops-on-one-core-unless-on-cores.md), Kavach spread eligible loops over all cores by itself, so the column "Now" for `loop`, `lists`, `closures` and `sort` is Kavach on all cores:

| Benchmark | First ratio | Now | Kavach got faster by |
|---|---|---|---|
| `matrix` | 51.0 | 0.47 | 108× |
| `records` | 14.79 | 0.31 | 47.7× |
| `closures` | 2.80 | 0.08 | 35.0× |
| `fib` | 28.0 | 0.81 | 34.6× |
| `maps` | 12.26 | 0.73 | 16.8× |
| `text` | 7.79 | 0.60 | 13.0× |
| `lists` | 3.78 | 0.33 | 11.5× |
| `sort` | 1.26 | 0.10 | 12.6× |
| `loop` | 1.12 | 0.03 | 37.3× |
| `floats` | 1.00 | 0.99 | — |


Each row is one improvement, with the ratio (Kavach ÷ C++) after it. Lower is better; below 1.00 Kavach wins.

| # | Date | Change | loop | fib | lists | matrix | records | maps | sort | closures | floats | text |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2026-10-31 | First native build | 1.08 | 28.0 | 3.78 | | | | | | | |
| 1 | 2026-10-31 | Call depth counted in generated code, not the runtime | 1.08 | 5.35 | 3.78 | | | | | | | |
| 2 | 2026-10-31 | More benchmarks; heap objects get a fixed layout | 1.12 | 5.35 | 2.79 | 51.0 | 14.79 | 12.26 | 1.26 | | | |
| 3 | 2026-10-31 | List items, fields and reference counts read in generated code | 1.19 | 5.59 | 2.86 | 5.13 | 15.46 | 12.10 | 1.54 | 2.80 | 1.00 | |
| 4 | 2026-10-31 | Maps: open-addressing table, fast seeded hash | 1.11 | 5.43 | 2.63 | 5.54 | 13.40 | 5.67 | 1.56 | 2.63 | 1.11 | |
| 5 | 2026-10-31 | Value ranges: proven-safe arithmetic has no checks; `%` by a constant becomes a compare and a subtract | **0.35** | 5.65 | 2.91 | 5.85 | 12.59 | 6.16 | 1.43 | 2.77 | 0.95 | |
| 6 | 2026-10-31 | Records, tuples and closures: one allocation with the fields inline; freeing allocates nothing | 0.42 | 6.34 | 1.55 | 5.57 | 6.55 | 5.96 | 1.39 | 1.55 | 0.99 | |
| 7 | 2026-10-31 | Stack overflow found by comparing the stack pointer with a limit, instead of counting calls in memory | 0.39 | 3.70 | 1.70 | 5.43 | 6.73 | 5.70 | 1.39 | 1.46 | 0.97 | |
| 8 | 2026-10-31 | Nested reads like `b[k][j]` and `for` variables borrow instead of counting references; an index out of range is a cold path that never rejoins | 0.37 | 3.54 | 1.64 | 3.47 | 6.47 | 5.80 | 1.38 | 1.29 | 0.95 | |
| 9 | 2026-10-31 | `m[k] ?? 0` is one lookup with nothing boxed (4.01 on maps) | 0.39 | 3.48 | 1.69 | 3.53 | 6.96 | 4.01 | 1.36 | 1.37 | 1.00 | |
| 10 | 2026-10-31 | Integer keys: cheap seeded hash that keeps sequential keys together, switching to the strong hash on any long probe ([design](../../docs/architecture/hash-maps.md)) | 0.38 | 3.56 | 1.75 | 3.50 | 7.21 | **0.99** | 1.37 | 1.39 | 1.04 | |
| 11 | 2026-10-31 | Records and tuples: a 24-byte header with the fields right after it (a 2-field record is 40 bytes, was 64); freeing skips types that hold no references | 0.39 | 4.02 | 1.62 | 3.57 | 5.19 | 0.91 | 1.39 | 1.29 | 1.01 | |
| 12 | 2026-10-31 | Small records, tuples and closures come from a pool: free lists per size, cut from large blocks | 0.39 | 3.15 | 1.59 | 3.77 | 3.03 | 1.06 | 1.35 | 1.36 | 1.02 | |
| 13 | 2026-10-31 | Native `split` (text 7.79); text methods read without copying, formatting appends in place, text objects from the pool | 0.39 | 3.37 | 1.85 | 4.17 | 3.02 | 0.96 | 1.46 | 1.45 | 1.05 | 4.85 |
| 14 | 2026-10-31 | Sort in place when nothing else holds the list; integers use the unstable pdqsort (equal integers can't be told apart) | 0.37 | 3.43 | 2.11 | 3.64 | 3.20 | 1.10 | **0.34** | 1.23 | 0.98 | 4.54 |
| 15 | 2026-10-31 | Loops read a list's buffer and length once when the body never changes the list | 0.37 | 3.61 | 1.85 | 4.17 | 3.08 | 1.06 | 0.33 | 1.56 | 1.02 | 4.81 |
| 16 | 2026-10-31 | Interpolated text built in place (text 3.77); text up to 30 bytes stored inside its object, no heap buffer | 0.38 | 3.28 | 1.40 | 3.43 | 3.31 | 0.98 | 0.32 | 1.32 | 1.01 | 2.89 |
| 17 | 2026-10-31 | Small functions are inlined into their callers, a recursive one into itself once; early returns become `if`/`else` | 0.39 | 2.71 | 1.63 | 3.42 | 3.34 | 1.01 | 0.33 | 1.68 | 1.00 | 2.91 |
| 18 | 2026-10-31 | `items.push(x)`, `items[i] = x` and `p.x = v` written back to the same variable use the in-place fast path (push used to always call the runtime) | 0.38 | 2.73 | **0.81** | 3.64 | 2.59 | 1.08 | **0.28** | **0.65** | 1.00 | 2.85 |
| 19 | 2026-10-31 | `join` writes each item into one result; `len()` of text reads it in place, ASCII fast path | 0.38 | 2.69 | 0.76 | 3.73 | 2.59 | 1.05 | 0.28 | 0.65 | 0.98 | 2.04 |
| 20 | 2026-10-31 | The object pool takes 4 MB blocks aligned to 2 MB, so Linux backs them with huge pages: 3,800 page faults instead of 101,000 on `records` | 0.38 | 2.68 | 0.65 | 3.58 | 1.98 | 0.98 | 0.27 | 0.61 | 0.99 | 1.81 |
| 21 | 2026-10-31 | Records, tuples and union values are taken from the pool by generated code: pop a free object or cut from the block, write the header, no call | 0.41 | 2.77 | 0.74 | 3.58 | 1.50 | 0.94 | 0.27 | 0.62 | 0.98 | 1.83 |
| 22 | 2026-10-31 | `if` conditions narrow integer ranges (`n - 1` in the `else` of `if n < 2` needs no check) | 0.40 | 2.88 | 0.79 | 3.50 | 1.46 | 1.00 | 0.26 | 0.61 | 0.94 | 1.75 |
| 23 | 2026-10-31 | `for` loops test once before the loop, then at the bottom of each round: one branch per round, and fewer registers spilled | 0.41 | 2.87 | 0.66 | **2.17** | 1.40 | 1.05 | 0.27 | 0.54 | 0.98 | 1.86 |
| 24 | 2026-10-31 | Innermost loops check list bounds once and run an unchecked copy (matrix about 5%); `m[k] ?? d` and `m[k] = v` call the map directly, not through the generic dispatcher | 0.38 | 2.89 | 0.60 | 2.23 | 1.45 | **0.71** | 0.25 | 0.58 | 0.98 | 1.90 |
| 25 | 2026-10-31 | Freeing a list of objects that hold no references (records of numbers, text) frees each item directly | 0.37 | 2.91 | 0.54 | 2.25 | 1.30 | 0.69 | 0.27 | 0.55 | 1.00 | 1.78 |
| 26 | 2026-10-31 | `main` skips freeing its objects one by one at exit (the system takes back all memory; leak checks still free them); text and other data objects are one 64-byte cache line (were 72 bytes); `split` on one byte uses a byte search and sizes the list first | 0.39 | 2.35 | 0.62 | 2.14 | **1.03** | 0.59 | 0.29 | 0.58 | 1.00 | 1.51 |
| 27 | 2026-10-31 | Flat lists: a list of plain records keeps their fields in its own buffer, 16 bytes per point instead of a pointer and a 40-byte object ([design](../../docs/architecture/flat-lists.md)); a `for` variable that only reads fields borrows, even on its last use | 0.40 | 2.37 | 0.64 | 2.25 | **0.45** | 0.60 | 0.28 | 0.58 | 0.98 | 1.51 |
| 28 | 2026-10-31 | Integer lists sort by radix: one pass splits them by the top byte into 256 buckets, then each bucket is sorted digit by digit inside the cache | | | | | | | **0.18** | | | |
| 29 | 2026-10-31 | Lists of text keep text up to 15 bytes inside each 16-byte item ([design](../../docs/architecture/flat-lists.md#lists-of-text)): `split` makes no objects, `join` copies 16 bytes per word, `len()` reads the item's tag | | | | | | | | | | **0.82** |
| 30 | 2026-10-31 | A function's quick exit (`if n < 2 { n }`) is copied in front of each call, so the leaves of a recursion, half of all calls, need no call | | **2.02** | | | | | | | | |
| 31 | 2026-10-31 | A small recursive function is inlined into itself two levels deep (2 calls become 16), while it stays under 400 expressions | | **1.77** | | | | | | | | |
| 32 | 2026-10-31 | LLVM backend for `kavach build` ([decision 0018](../../docs/decisions/0018-llvm-backend-through-llvm-tools.md)): numbers, control flow, calls and lists of numbers go through `opt -O3` and `llc`; other programs still use Cranelift | **0.31** | **0.87** | 0.62 | **1.87** | 0.46 | 0.62 | 0.18 | 0.59 | 1.01 | 0.85 |
| 33 | 2026-10-31 | LLVM backend: `for` over lists, and built-in methods through the runtime (`sort`, `map`, `filter`, `sum` now go through LLVM) | | | | | | | **0.17** | **0.51** | | |
| 34 | 2026-10-31 | LLVM backend: records, tuples and flat lists of records (fields read in the list's buffer, `push(Point(…))` with no object) | | | | | **0.36** | | | | | |
| 35 | 2026-10-31 | LLVM backend: text, interpolation, `==`/`+`/ordering on heap values, and lists of text (`w.len()` read from the item's tag) | | | | | | | | | | **0.80** |
| 36 | 2026-10-31 | Unroll and jam: a loop that sums an inner loop computes 8 cells at once, sharing each row lookup, after one check that covers all their indexes; otherwise the plain loop runs | | | | **0.93** | | | | | | |
| 37 | 2026-10-31 | Maps with integer keys: `m[k] = v` probes once, and the table keeps no hashes (cheaper to compute again) | | | | | | **0.48** | | | | |
| 38 | 2026-10-31 | ``list.push(`word{i}`)`` on a list of text builds the text right in a 16-byte item on the stack: no text object for short text | | | | | | | | | | **0.62** |
| 39 | 2026-10-31 | List pipelines fused: `map`, then `filter`, then `sum` run as one loop and build no lists in between; if a check fails, the code as written runs instead, so panics stay where the program says | | | | | | | | **0.26** | | |
| 40 | 2026-10-31 | Unroll and jam: the check before the loop proves every index of the 8 cells, so the inner loop reads without checks and LLVM turns it into vector instructions | | | | **0.56** | | | | | | |

How step 14 beats C++ on `sort`: `items` is not used after `items.sort()`, so Kavach sorts the list itself; the C++ program copies it first, as C++ code must when the original is still needed. The value-range pass also proves the random numbers are never negative, so `% 2147483648` becomes a bit mask.

How step 5 beats C++ on `loop`: Kavach proves that `total` always stays between 0 and the modulus, so `(total + x) % m` needs one compare and one subtract instead of a division. `g++` doesn't track value ranges across loop rounds, so it divides every round (as a multiplication by a "magic" constant, which is still about 10 cycles on the loop's critical path).

## Running them

From the repository root, on Linux or WSL with `g++` installed:

```
sh scripts/run-benchmarks.sh
```

On Windows, `powershell -File scripts/run-benchmarks.ps1` runs the same script in WSL.

With `go` and `dotnet` on the path (or `GO` and `DOTNET` pointing at them), the script times the Go and C# programs too. For `loop`, `sort` and `lists` it prints two rows, `(1 core)` and `(all cores)`, and builds the all-core twins with `g++ -O2 -fopenmp`.
