# benchmarks/tasks

Tasks that only compute, run one by one and with `parallel_map` ([spec 9](../../docs/spec/09-concurrency.md), [decision 0020](../../docs/decisions/0020-tasks-take-turns.md)). In a program built by `kavach build`, a new task computes ahead on its own core, on its own copies of its values, and waits for its turn only before its first visible effect, so the output is the same as when tasks take turns.

| File | What it measures |
|---|---|
| [kavach/parallel-map.kav](kavach/parallel-map.kav) | Eight sums of 150 million rounds each: `map`, then `parallel_map` |
| [cpp/parallel-map.cpp](cpp/parallel-map.cpp) | The same in C++: one by one, then a `std::thread` per sum |

## Results

Measured 2026-11-10 at commit 567af3f with the loop copies below (WSL Ubuntu 24.04, g++ 13 `-O2`, i7-13700HX: 24 threads, 8 fast cores and 8 slower ones, [the machine](../README.md#the-machine)). `kavach build` (LLVM backend). Each program prints its own times; three rounds of three runs, Kavach and C++ in turns, the best of all nine. The machine was busier than on 2026-11-09, when C++ took 3,402 ms one by one and 470 ms with threads, so compare the two columns, not the numbers with older ones.

| Way | Kavach ms | C++ ms |
|---|---|---|
| `map` (one by one) | 4456 | 4788 |
| `parallel_map` | 677 | 776 |

The best of each round, Kavach and C++: one by one 4476 and 5001, 4608 and 4788, 4456 and 4870; `parallel_map` 701 and 778, 677 and 776, 695 and 783.

`parallel_map` is 6.6× faster than one by one with 8 tasks: each sum runs on a core of its own, and the slowest core sets the time. Kavach is now about 7% faster than C++ one by one and 13% with threads. Each round waits for the remainder of the round before, and C++ divides `total`, a signed `long long`, with its sign. Since 2026-11-10 Kavach runs a copy of the loop where the seed is between 0 and 2³¹ and `total` isn't negative (`compiler/kir/src/version.rs`). The copy has no overflow checks, and its locals are its own, so Kavach knows `total` stays between 0 and 1,000,000,006 and divides without a sign. On 2026-11-09, without the copy, Kavach was about 4% slower than C++.

**Found while measuring:** from [decision 0069](../../docs/decisions/0069-each-task-chooses-how-it-runs.md) on, a plain `spawn` is a coroutine, and `parallel_map` started its tasks the same way, so all eight sums ran on one core: 3,666–3,904 ms, no faster than one by one. Its tasks are now threads, as `spawn thread` makes, which compute ahead on cores of their own until their first effect ([decision 0048](../../docs/decisions/0048-tasks-as-coroutines-per-core.md) kept `parallel_map` on all cores); a unit test in `compiler/kir/src/lower_tasks/parallel_map.rs` checks it.
