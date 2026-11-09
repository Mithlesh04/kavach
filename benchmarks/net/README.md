# benchmarks/net

How fast a task waiting for the network wakes ([spec 14.10](../../docs/spec/14-standard-library/README.md), [decision 0046](../../docs/decisions/0046-io-event-loop-through-mio.md)): two tasks pass 8 bytes back and forth 10,000 times over TCP on this machine. Each round trip waits for the network twice, so the time is mostly waking up.

| File | What it is |
|---|---|
| [kavach/pingpong.kav](kavach/pingpong.kav) | A spawned server task echoes what a client task sends |
| [cpp/pingpong.cpp](cpp/pingpong.cpp) | The same with blocking sockets and two threads, as most C++ programs write it |
| [python/pingpong.py](python/pingpong.py) | The same with blocking sockets and a thread |
| [kavach/clients.kav](kavach/clients.kav) | An echo server with 100 clients at once, 100 round trips each: a task per connection |
| [cpp/clients.cpp](cpp/clients.cpp) | The same with a thread per connection on both sides, blocking sockets |
| [python/clients.py](python/clients.py) | The same with Python threads |

Run them (Linux or WSL):

```
sh scripts/run-benchmarks.sh "" benchmarks/net
```

## Results

**Now:** measured 2026-11-09 at commit ac2657be (`kavach --version`; [the machine](../README.md#the-machine)), in three rounds of best of 9 (Python: best of 3) one after another; each figure is the best of the three rounds, in ms. Linux is WSL Ubuntu 24.04 through `scripts/run-benchmarks.sh`; there is no C++ build on Windows.

| Benchmark | Linux: Kavach | Linux: C++ | Linux: Python | Windows: Kavach | Windows: Python |
|---|---|---|---|---|---|
| `pingpong` | 57 | 345 | 423 | 285 | 442 |
| `clients` (100 clients × 100 trips) | 135 | 42 | 1,099 | 305 | 353 |

Kavach's time over C++'s on Linux in each round: `pingpong` 0.17, 0.16, 0.15; `clients` 2.73, 2.71, 3.21. With a plain `spawn` per connection, as `clients.kav` writes it, the 100 clients' tasks share one core, and C++ uses all of them; `spawn cores` and the C++ program on one core are compared below. The tables below show how it got here.

Measured 2026-11-03, i7-13700HX, in ms. Linux is WSL Ubuntu 24.04, g++ 13 `-O2`, Python 3.12; Windows is Windows 11 on the same machine.

| Way | Linux | Windows |
|---|---|---|
| Kavach, trying again after a 1–8 ms sleep (before) | 11,900 | 156,000 |
| Kavach, woken by a poller thread (first try) | 1,150 | 640 |
| Kavach, waiting on the readiness queue itself | 430 | 600 |
| **Kavach, waking only the next task, which spins a moment first** | **110** | **470** |
| C++, blocking sockets | 300 | |
| Python, blocking sockets | 448 | 390 |

**What the numbers say:** before, every wait slept at least 1 ms, and on Windows a 1 ms sleep lasts a whole 15.6 ms timer tick. Now a waiting task sleeps until the system says its socket is ready. With a poller thread of its own, each wake-up went through three threads; with the scheduler's thread waiting on the queue itself, it goes through one switch, as in C++. Then the turn passing between tasks (decision 0020) got cheaper: only the next task's thread is woken, and a thread whose turn may come spins for up to 50 µs before it sleeps (at most 4 threads at once), so it sees its turn in well under a microsecond (`runtime/core/src/tasks/turns.rs`). On Linux, Kavach is now 2.8× faster than C++ with blocking sockets: the kernel puts C++'s reader to sleep and wakes it on every trip. On Windows each socket call costs more, and Kavach makes about twice as many as a blocking read (a try that would block, then the readiness queue), so there it is a little slower than Python.

### Many clients at once (`clients`)

Measured 2026-11-03, Linux (WSL), in ms:

| Way | 100 trips each | 1,000 trips each |
|---|---|---|
| Kavach, a thread per task | 920 | 5,500 |
| **Kavach, a coroutine per task** ([decision 0048](../../docs/decisions/0048-tasks-as-coroutines-per-core.md), step 1) | **121** | **1,040** |
| Kavach, `spawn cores` on 24 workers (2026-11-06; [measured here](../../docs/architecture/tasks-on-all-cores.md#measured-so-far)) | 90 | 257 |
| Kavach, `spawn cores`, each worker looking at its sockets before it sleeps (2026-11-07, a slower day: C++ took 267–298 ms then; [measured here](../../docs/architecture/tasks-on-all-cores.md#measured-so-far)) | | 386–435 |
| C++, all 24 cores | 20 | 93 |
| C++, one core (`taskset -c 0`) | | 345 |
| Python | 1,180 | |

**What the numbers say:** with a thread per task, Kavach was far behind C++. Its tasks take turns, one at a time ([decision 0020](../../docs/decisions/0020-tasks-take-turns.md), stage 1), while C++ runs 200 threads on 24 cores. And each turn passes between operating-system threads: with 200 tasks, a thread waits through many turns before its own, falls asleep, and the system takes tens of microseconds to wake it on another core. Even on one core, C++ is 16× faster, because its threads switch on the same core. Kept on one core with `taskset`, Kavach falls to 1,741 ms on Linux, but on Windows it gets slower (6.9 s → 15.8 s), so that is no general fix. Now each task is a coroutine on the main thread, and passing the turn switches stacks without the operating system: 5.3× faster. C++ is still ahead: it runs on 24 cores, and even on one core its blocking reads make fewer system calls per trip than Kavach's try-then-wait. Tasks on all cores are decision 0048's next steps.

**Profiled on one core** (2026-11-05, WSL, 100 clients × 1,000 trips, `samply`): Kavach 0.72 s (user 0.29 s, system 0.36 s), C++ on one core 0.48 s (user 0.03 s, system 0.38 s), C++ on all cores 0.19 s. The system time is the same; the difference is 0.25 s of user time, about 1.25 µs a round trip. In the profile, `send` and `recv` themselves take about 55% (the kernel's work, which C++ does too), and the rest is spread thin over the scheduler's wait (its turn, the poller's lock and `HashMap`, coroutine switches, thread-locals), no function above 2%. The tries that would block are not the cost: skipping a read when no event came since the socket was emptied (as Tokio's readiness flags do) took half the reads off and changed nothing measurable, so it was not kept. Closing the gap means a shorter wait path as a whole: the poller's registration without a lock and a SipHash map, and fewer layers between `conn.read()` and the coroutine switch.

