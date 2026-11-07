# benchmarks/tasks

Tasks that only compute, run one by one and with `parallel_map` ([spec 9](../../docs/spec/09-concurrency.md), [decision 0020](../../docs/decisions/0020-tasks-take-turns.md)). In a program built by `kavach build`, a new task computes ahead on its own core, on its own copies of its values, and waits for its turn only before its first visible effect, so the output is the same as when tasks take turns.

| File | What it measures |
|---|---|
| [kavach/parallel-map.kav](kavach/parallel-map.kav) | Eight sums of 150 million rounds each: `map`, then `parallel_map` |

## Results

Linux (WSL Ubuntu 24.04), Intel i7-13700HX (24 threads: 8 fast cores, 8 slower ones). `kavach build` (LLVM backend). Typical of 3 runs.

| Way | ms |
|---|---|
| `map` (one by one) | 3970 |
| `parallel_map` | 670 |

`parallel_map` is 5.9× faster with 8 tasks: each sum takes about 490 ms on a fast core, and the slowest core sets the time.

A comparison with C++ threads comes with the `math` benchmarks (to-do item 1).
