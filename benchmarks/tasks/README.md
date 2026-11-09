# benchmarks/tasks

Tasks that only compute, run one by one and with `parallel_map` ([spec 9](../../docs/spec/09-concurrency.md), [decision 0020](../../docs/decisions/0020-tasks-take-turns.md)). In a program built by `kavach build`, a new task computes ahead on its own core, on its own copies of its values, and waits for its turn only before its first visible effect, so the output is the same as when tasks take turns.

| File | What it measures |
|---|---|
| [kavach/parallel-map.kav](kavach/parallel-map.kav) | Eight sums of 150 million rounds each: `map`, then `parallel_map` |
| [cpp/parallel-map.cpp](cpp/parallel-map.cpp) | The same in C++: one by one, then a `std::thread` per sum |

## Results

Measured 2026-11-09 at commit 38fb662d with the `parallel_map` fix below (WSL Ubuntu 24.04, g++ 13 `-O2`, i7-13700HX: 24 threads, 8 fast cores and 8 slower ones, [the machine](../README.md#the-machine)). `kavach build` (LLVM backend). Each program prints its own times; three rounds of three runs, the best of all nine.

| Way | Kavach ms | C++ ms |
|---|---|---|
| `map` (one by one) | 3555 | 3402 |
| `parallel_map` | 524 | 470 |

The best of each round, Kavach and C++: one by one 3630 and 3435, 3555 and 3402, 3711 and 3549; `parallel_map` 524 and 492, 527 and 490, 549 and 470.

`parallel_map` is 6.8× faster than one by one with 8 tasks: each sum takes about 450 ms on a fast core, and the slowest core sets the time. C++'s threads do the same in 470 ms; Kavach's sums are about 5% slower one by one, as each round checks its arithmetic for overflow.

**Found while measuring:** from [decision 0069](../../docs/decisions/0069-each-task-chooses-how-it-runs.md) on, a plain `spawn` is a coroutine, and `parallel_map` started its tasks the same way, so all eight sums ran on one core: 3,666–3,904 ms, no faster than one by one. Its tasks are now threads, as `spawn thread` makes, which compute ahead on cores of their own until their first effect ([decision 0048](../../docs/decisions/0048-tasks-as-coroutines-per-core.md) kept `parallel_map` on all cores); a unit test in `compiler/kir/src/lower_tasks/parallel_map.rs` checks it.
