# benchmarks/tasks

Tasks that only compute, run one by one and with `parallel_map` ([spec 9](../../docs/spec/09-concurrency.md), [decision 0020](../../docs/decisions/0020-tasks-take-turns.md)). In a program built by `kavach build`, a new task computes ahead on its own core, on its own copies of its values, and waits for its turn only before its first visible effect, so the output is the same as when tasks take turns.

| File | What it measures |
|---|---|
| [kavach/parallel-map.kav](kavach/parallel-map.kav) | Eight sums of 150 million rounds each: `map`, then `parallel_map` |
| [cpp/parallel-map.cpp](cpp/parallel-map.cpp) | The same in C++: one by one, then a `std::thread` per sum |

## Results

Measured 2026-11-10 at commit 6654f22 with the remainder change below (WSL Ubuntu 24.04, g++ 13 `-O2`, i7-13700HX: 24 threads, 8 fast cores and 8 slower ones, [the machine](../README.md#the-machine)). `kavach build` (LLVM backend). Each program prints its own times; three rounds of three runs, Kavach and C++ in turns, the best of all nine. The machine was busy, as on the earlier measurements of the day, so compare the two columns, not the numbers with older ones.

| Way | Kavach ms | C++ ms |
|---|---|---|
| `map` (one by one) | 164 | 4261 |
| `parallel_map` | 210 | 554 |

The best of each round, Kavach and C++: one by one 164 and 4306, 164 and 4261, 165 and 4409; `parallel_map` 231 and 568, 210 and 554, 218 and 741.

Kavach is 26× faster than C++ one by one and 2.6× with `parallel_map`. Each sum is `total = (total + i * seed) % 1_000_000_007`, and C++ waits for a division in every round, since each round needs the remainder of the round before. Kavach, where `total` is known to lie between 0 and 1,000,000,006 and `i * seed` is never negative, works out `total + (i * seed) % 1_000_000_007` instead, less 1,000,000,007 when it reaches that: the same number, but the division no longer waits for the round before, which only waits for an add and a compare (`compiler/kir/src/ranges/arith.rs`). That makes the loop a sum taken by a remainder, which runs on all cores ([decision 0031](../../docs/decisions/0031-loops-on-all-cores.md)): so in Kavach "one by one" means one sum after another, each spread over every core, while C++'s one by one is one thread. With `parallel_map` both use a thread per sum; there, when several sums run at once, each stays on its own thread instead of waiting for the pool (`runtime/native/src/rt_par.rs`). The copy of the loop this needs, without overflow checks and run while the seed is between 0 and 2³¹, comes from `compiler/kir/src/version.rs`. Before these changes (2026-11-09), Kavach was about 4% slower than C++.

**Found while measuring:** from [decision 0069](../../docs/decisions/0069-each-task-chooses-how-it-runs.md) on, a plain `spawn` is a coroutine, and `parallel_map` started its tasks the same way, so all eight sums ran on one core: 3,666–3,904 ms, no faster than one by one. Its tasks are now threads, as `spawn thread` makes, which compute ahead on cores of their own until their first effect ([decision 0048](../../docs/decisions/0048-tasks-as-coroutines-per-core.md) kept `parallel_map` on all cores); a unit test in `compiler/kir/src/lower_tasks/parallel_map.rs` checks it.
