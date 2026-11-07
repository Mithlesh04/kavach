# benchmarks/channels

Passing values between tasks ([spec 9.4](../../docs/spec/09-concurrency.md#94-channels)): one task sends 2 million integers through a channel that holds 1024, and the receiver adds them up. Kavach, C++ (`g++ -O2`) and Python do the same work.

| File | What it is |
|---|---|
| [kavach/pipe.kav](kavach/pipe.kav) | `Channel<Int>(capacity: 1024)`, a spawned sender, a `for` loop receiving |
| [cpp/pipe.cpp](cpp/pipe.cpp) | A bounded queue as C++ programs usually write one: a mutex and two condition variables (C++ has no channel), a sender thread |
| [python/pipe.py](python/pipe.py) | `queue.Queue(maxsize=1024)` and a sender thread |

Run them (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/channels
```

## Results

Measured 2026-11-02, Kavach again 2026-11-03 (WSL Ubuntu, g++ 13, Python 3.12, i7-13700HX), best of 9 (Python: best of 3); ratio is Kavach ÷ C++, so below 1 is faster than C++.

| Benchmark | Kavach ms | C++ ms | Ratio | Python ms | × Python |
|---|---|---|---|---|---|
| `pipe` | 79 | 474 | **0.17** | 4560 | **58×** |

**What the numbers say:** the turn passes to one waiting thread, which spins for a moment before it sleeps (`runtime/core/src/tasks/turns.rs`); before that change it took 303 ms. Kavach's tasks take turns ([spec 9.7](../../docs/spec/09-concurrency.md#97-taking-turns)), so only one task touches a channel at a time and the channel itself needs no lock (`runtime/native/src/rt_channels.rs`). The C++ queue pays a mutex and a condition-variable signal per value, the way most C++ code is written; a lock-free single-producer ring buffer would be faster than both, and would be the fair next comparison. Python's `queue.Queue` pays a lock and the interpreter on every value.
