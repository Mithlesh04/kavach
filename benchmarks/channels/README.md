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

Measured 2026-11-09 at commit 38fb662d (`kavach --version`; WSL Ubuntu, g++ 13, Python 3.12, i7-13700HX, [the machine](../README.md#the-machine)), in three rounds of best of 9 (Python: best of 3) one after another; the row is the best of the three rounds. Ratio is Kavach ÷ C++, so below 1 is faster than C++.

| Benchmark | Kavach ms | C++ ms | Ratio | Python ms | × Python |
|---|---|---|---|---|---|
| `pipe` | 69 | 234 | **0.29** | 2418 | **35×** |

The ratio in each round, in order: 0.31, 0.29, 0.29. On 2026-11-03 Kavach took 79 ms against C++'s 474 and Python's 4,560: C++ and Python run this pipe twice as fast on this day, Kavach a little faster.

**What the numbers say:** the turn passes to one waiting thread, which spins for a moment before it sleeps (`runtime/core/src/tasks/turns.rs`); before that change it took 303 ms. Kavach's tasks take turns ([spec 9.7](../../docs/spec/09-concurrency.md#97-taking-turns)), so on one core only one task touches a channel at a time; for tasks on all cores a channel's state has a short spin lock (`runtime/native/src/rt_channels.rs`). The C++ queue pays a mutex and a condition-variable signal per value, the way most C++ code is written; a lock-free single-producer ring buffer would be faster than both, and would be the fair next comparison. Python's `queue.Queue` pays a lock and the interpreter on every value.

**Since then** (2026-11-09, side by side in turns on WSL, best of 7, the pipe with 20 million values): 1,272 → 759 ms. Measured three times at commit b65b7ae7, the 2-million pipe above took 116–121 ms against C++'s 218–267 (Python 2,409–2,680): slower than on 2026-11-03, as a channel's state had gained a lock for tasks on all cores (decision 0049), and every take an optional made only to be unwrapped. Two changes took that back: the lock is a spin lock, one atomic exchange to take and a store to let go, where a `Mutex` lets go with another exchange (`runtime/native/src/rt_channels.rs`; 1,306 → 1,060 ms; 48 producers on 24 workers the same); and `for v in receive` over values held by value takes each value itself, with no optional (`RtOp::ChanTakeValue`; 1,060 → 759 ms). On Windows the spin lock alone: 1,652 → 1,311 ms. With both, at commit 38fb662d, the 2-million pipe takes 69–82 ms (the table above).
