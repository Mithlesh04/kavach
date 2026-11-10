# Tasks on all cores: the proposal

**Status: accepted as [decision 0049](../decisions/0049-tasks-on-all-cores-keep-the-programs-order.md)** (Mithlesh, 2026-11-03: rule (a), all cores by default, one core kept). It is decision [0048](../decisions/0048-tasks-as-coroutines-per-core.md)'s steps 2 and 3 worked out: what changes for programs, and how to build it.

**Since [decision 0069](../decisions/0069-each-task-chooses-how-it-runs.md)** (2026-11-05), all cores are not the default and `tasks =` is no setting of `kavach.toml`: each task asks for them with `spawn cores`, and a plain `spawn` stays a coroutine. Where this page says `tasks = "parallel"`, read a program whose tasks use `spawn cores`.

## Where things stand

Tasks are coroutines on the main thread ([decision 0048](../decisions/0048-tasks-as-coroutines-per-core.md), step 1). One task runs at a time and the turn passes in a defined order (spec 9.7), so a program prints the same every run, and the interpreter and native code print the same. Native code is fast at passing the turn (100 clients × 1,000 TCP round trips: 1.04 s), but it uses one core: C++ with a thread per connection takes 93 ms on 24 cores ([benchmarks/net](../../benchmarks/net/README.md#many-clients-at-once-clients)).

Some groundwork exists. Values are copied into a task (spec 9.5, `runtime/native/src/send.rs`). Shared values (`Shared`, channels, task results) are objects around a cell with an atomic count. Each thread has its own allocation pool when tasks run in parallel (`pool.rs`). What a cell holds, though, is used only by the task holding the turn: that is the assumption parallel tasks break.

## What Mithlesh chose

What tasks **print and log** stays in order; the network, files, channels and timers run in parallel (Mithlesh, 2026-11-03).

## The rule, precisely

"In order" can mean two things once tasks really run at the same time. They cost very differently.

**(a) The order the program says.** A task's own lines come out in the order it prints them. Between tasks, a line printed before a task sends a value comes before the lines the receiver prints after taking it (and the same for spawning a task and waiting for one). Lines no such link orders may come out either way, but never mixed into one line. Go, Rust and Java work this way.

**(b) The interpreter's order, always.** Native code runs tasks in parallel, but holds each line back until the interpreter's schedule says it is that line's turn. Output is repeatable for every program, but each print waits for the schedule. Since the schedule depends on when tasks wait, every task's progress would be tracked in a logical clock (deterministic multithreading). That is slow, and a large amount of work.

**Recommended: (a), with one-core runs kept for repeatability.** `tasks = "coroutines"` (spec 9.8) would keep today's one-core turn order, which is (b) for free: tests and the interpreter compare line by line there. The default becomes all cores with rule (a). This needs a third value for `tasks`, and a new default: a maintainer's choice.

## How to build it, in steps

Each step lands on its own, with its benchmark.

1. **Cells for threads.** Channels, `Shared`, task results, agents' state and `std.log` take a lock (or are lock-free queues) instead of relying on the turn. A value going into a cell is copied in, as a task's closure already is.
2. **Memory across threads.** Each worker thread allocates from its own pool, as now. An object freed on another thread goes back to its owner's pool through a list the owner empties (as mimalloc does), because reference counts stay non-atomic: no two threads ever hold the same object, since values cross only as copies.
3. **Workers.** One thread per core, each running coroutines. A new task goes to the worker with the least to do. A coroutine stays on its worker: moving it would move objects of one pool to another thread's code. Each worker waits on the readiness queue for its own sockets, and `parallel_map` keeps its own pool of threads.
4. **Ordering lines.** Under rule (a), printing takes a process-wide lock for one line at a time. The links of rule (a) already hold: sending on a channel and spawning a task synchronize memory.
5. **Generated code.** The Cranelift JIT reads a global stack limit and pool; they become per worker, as the LLVM build's already are.

## Measured so far

A first version (2026-11-03, not on `main` yet): one scheduler, with a turn and a ready queue for each worker, behind one lock; a thread of its own for the readiness queue. 100 clients × 1,000 round trips, Linux:

| Workers | ms |
|---|---|
| 1 (coroutines) | 1,084 |
| 4 | 897 |
| 24 | 2,427 |

Four workers beat one, but 24 are slower than one: every turn, on every worker, takes the scheduler's one lock. So step 3 needs a scheduler per worker, each with its own lock and queue, and wake-ups between workers through a queue per worker, as Go and tokio do.

Then (2026-11-03) each worker got a shard of the scheduler (its tasks, ready queue and timers, under its own lock; a task's id says its worker, and nothing holds two shards' locks at once) and an inbox of its own. 24 workers: 2,427 → 1,480 ms. Then each worker got its own readiness queue, waited on when it has nothing to run, in place of the one thread for all sockets; a wake-up for another worker signals it only when that worker sleeps, the way it sleeps (`Sleep` in `runtime/core/src/tasks.rs`). 24 workers: 1,480 → 1,050 ms, level with one core (1,025 ms in the same run).

Level was not faster, and the reason was in the code: a socket joined the queue of the worker that made it. `serve` accepts every connection, so all 100 server-side sockets sat on its worker's queue, while their handlers ran on other workers. Each message to the server woke `serve`'s worker, which then woke the handler's: two wake-ups, with one thread in the middle of all of them. Now a socket moves to the queue of the worker whose task waits on it, at that task's first wait (`move_here` in `runtime/net/src/net/poller.rs`). 24 workers: 1,050 → 380 ms, three times one core (1,148 ms in the same run). C++ on all cores: 93 ms.

CPU time shows what is left: 100,000 round trips take Kavach about 0.8 s of user time against C++'s 0.12 s, while their system time is close. A read allocated and zeroed 64 KB each time, even when it found nothing; one buffer per thread took one core from 1,052 to 769 ms. The rest needs a profiler (`samply` in WSL, which needs `perf_event_paranoid` at 1).

**Channels across cores** (2026-11-03): a channel's state is behind a lock, a value put in one is copied for the task that takes it, and waiters are woken by wait number, so a wake-up just before a task starts waiting still ends that wait. 48 producers and one consumer, 96,000 items: correct every run, but 760 ms on 24 workers against 60 ms on one core, since nearly every item wakes a thread on another core. A worker about to sleep first watched its ready queue for up to 50 µs (850 → 760 ms); measured again on 2026-11-06, it no longer helped, and it slowed the network (below), so it was removed.

**Printed lines follow rule (a)** (2026-11-03): with several workers, a `print`'s pieces collect in the task's own line, moved with it when coroutines switch, and the whole line is written in one call. 48 tasks printing 500 lines each: every line whole, every task's lines in order.

**`Shared` across cores** (2026-11-03, [decision 0052](../decisions/0052-shared-update-holds-its-lock-and-never-waits.md)): `update` holds the value's lock while its function runs, and that function may not wait (a compile error where the checker sees it, a panic otherwise), so the lock is held for nanoseconds and never across a wait. 48 tasks × 20,000 updates of one counter on 24 workers: always 960,000. With a lock that gave up its time slice after 100 tries it took 3.4 s; with exponential backoff, 0.35 s, against 0.2 s on one core.

**The conformance tests under `tasks = "parallel"`** (2026-11-03, `.kavach/probe/parsuite.py`): the 33 that use tasks, channels or `Shared`, built and run on 24 workers. One hung: with several workers, a worker with nothing to do slept, so no one saw that every task waited for another. Now the last worker to fall asleep looks at every shard, one lock at a time; with nothing ready, no timer and no socket awaited anywhere, it reports the deadlock, with the same message as on one core. Now 32 print as on one core, and `effects-keep-the-order-of-turns.kav` prints the same lines in another order, as rule (a) allows: it tests the one-core order of 9.7.

**Two cheap fixes for the channel, measured and dropped** (2026-11-04): letting half the workers spin before sleeping (instead of 4) took the channel test from 760 ms to 1.5–2.8 s, as the spinners fought over the shard locks; waking as many putters as there is room for, once the channel is half empty, took it to 1.3 s on 24 workers and from 60 to 300 ms on one core. What Go does instead is run the woken task on the waker's worker, so the hand-over needs no wake-up at all; here that means moving a coroutine between threads, which needs a profile first (`samply`, `perf_event_paranoid` at 1).

**The channel, profiled** (2026-11-04, `samply` on WSL, `.kavach/probe/profchan.sh`): on the consumer's thread, 90% of the samples are the futex system call that `take` makes to wake a producer's sleeping worker: about 12 µs each in WSL's virtual machine, once per item. Native Windows wakes a thread far cheaper: the same program takes about 250 ms on 24 workers against 200 ms on one core. Spinning only hides the wake-up: watching a flag instead of the lock changed nothing at 4 spinners (925 → 922 ms); 12 spinners for 1 ms gave 573 ms, every core for 5 ms 361 ms, but a worker that spins that long burns a core every time the program waits, so neither was kept. What is left is Go's way: the woken task runs next on the waker's worker, so no thread sleeps or wakes. That needs three things this runtime doesn't have yet: a coroutine that can move between threads (`corosensei`'s aren't `Send`: moving one is `unsafe`, with a `SAFETY` reason), memory freed on another thread going back to its pool (as mimalloc does), and no code that keeps a thread-local's address across a wait. That last one includes generated code: it takes small objects straight from its thread's pool (`pool.rs`), and LLVM may work out the pool's address once, before a loop that waits in it (`send.put` in a producer's loop), so a task moved in the loop would take objects from the old thread's pool. The pool's address would have to come from the task, not the thread, or be read again after every call that can wait.

**The network, profiled** (2026-11-06, `samply` on WSL, `.kavach/probe/profnet.sh` and `top2.py`, which names each frame by its library: an address in libc is not one in the program). First, two traps in measuring. The `timeout` command adds about 40 ms to every run of a Kavach program in WSL (not to C++), and a script without `~/.cargo/bin` on its `PATH` builds with Cranelift, whose programs never start workers, so `spawn cores` ran on one core. Measured without either, 100 clients × 1,000 round trips: plain `spawn` 629 ms, `spawn cores` 257 ms, C++ 108 ms. On 1 to 24 cores (`taskset`), Kavach stays about 2.2× behind C++ at every count, so the gap is the work per round trip, not how it spreads.

- **Workers no longer spin before they sleep.** The spin watched the ready queue, never the sockets, so a reply that came during it waited up to 50 µs. One client on all cores, 20,000 trips: 1,165 → 896 ms. 100 clients: user time 3.0 → 1.75 s, wall time the same. Channels (48 producers, a pipe): the same with or without it.
- **A socket object holds its connection** (`opaque_kind!`, as a regex does), in place of a numbered table behind one lock, which every read and write took: 6% of the samples, mostly waiting for that lock.
- **Tried and dropped:** a read that skips the try when no event came since the socket was found empty (as Tokio does) changed nothing, as on one core before.
- **A worker looks at its sockets before it sleeps** (2026-11-07, `LOOK` in `runtime/net/src/net/poller.rs`). A worker with nothing to run polls its queue for up to 100 µs, then sleeps on it. Unlike the old spin, this one watches the sockets, and a reply usually comes within it, so the worker skips going to sleep and being woken, which cost most in a virtual machine. Linux in WSL, best of 5 over 3 rounds, in ms:

  | How long it looks | 1 client, 20,000 trips | 100 clients, 1,000 trips each | Mostly idle server: CPU |
  |---|---|---|---|
  | Not at all | 2,364–2,593 | 683–889 | 198% |
  | 50 µs | 327–350 | 433–509 | 203% |
  | **100 µs** | **312–336** | **386–435** | **234%** |
  | 200 µs | 297–313 | 410–516 | 479% |
  | C++, blocking sockets | 2,068–2,286 | 267–298 | |

  CPU falls too on a busy server: 100 clients went from 15.6 s (13.3 in the kernel) to 7.5 s (4.3), where C++ uses 5.1 s. On 2 cores it changes little (980–1,020 ms before, 910–970 after; C++ 570–590), on 4 cores 700–770 → 600–660 (C++ 320–340). The "mostly idle server" is 100 clients waiting 5 ms between trips: past 100 µs a worker burns more than a reply saves, which is why the look stops there. One client beats C++ here because its thread sleeps in each `recv`: a C++ program that polled would gain the same. On Windows, with LLVM and on workers (29 threads or more), neither the time (100 clients 1,150–1,290 ms, 1 client 890–1,000 ms, at every length) nor the CPU moved, so the look costs nothing there and gains nothing: what takes the time on Windows is something else, not yet profiled.
- **The look gives up when it finds nothing** (2026-11-07, `Looks` in the same file). Replies that come just after the look make it cost CPU for nothing: a server with one client 200 µs apart went from 0.5 s to 1.1 s of CPU, with no gain in time. So after 8 looks in a row that find nothing, the thread sleeps at once, and looks again at every 16th sleep. That took the cost away, so plain `spawn` looks too. Side by side, best of 5 over 2 or 3 rounds, on a slower day, in ms:

  | Case | No look | Look that gives up | C++, blocking sockets |
  |---|---|---|---|
  | `spawn cores`, 1 client, 20,000 trips | 2,217–2,464 | 279–324 | 2,129–2,184 |
  | `spawn cores`, 100 clients, 1,000 trips each | 1,027–1,175 | 445–579 | 298–383 |
  | Plain `spawn` server, 1 client in another program, 20,000 trips | 2,064–2,343 | 621–702 | 1,992–2,185 |
  | Plain `spawn` server, 100 clients in another program, 1,000 trips each | 3,717–3,721 | 3,740–3,850 | |

  With one worker and 100 busy clients nothing changes: that worker never has nothing to do. Its CPU: one client 200 µs apart 0.36–0.44 s without a look, 0.40–0.42 s with it; 1 ms and 5 ms apart the same; 100 clients 5 ms apart the same. A mostly idle `spawn cores` program does pay: 197% → 274% of a core, 0.87 → 0.83 s.
- **What is left:** `send`'s time in the kernel is the same as C++'s. On top, each wait costs about 1.6 µs on one core: the read that finds nothing and the `epoll_wait`, two system calls a blocking read doesn't make (1 client: 1.38× C++; 100 clients: 1.59×). Plain `spawn` stays on one core: with 100 busy clients, 19–37 µs a round trip on the days measured, most of it in the kernel, where C++ has all cores. And each `net.connect` starts a thread of its own for the name lookup: 100 clients with one round trip take 30 ms with `spawn cores`, 11 ms with plain `spawn`. Keeping those threads for the next connect was tried (2026-11-10) and changed nothing measurable: they run beside the tasks, on other cores. What did matter was the listener's queue: Rust's `bind` keeps 128 connections waiting to be accepted, and the system dropped the rest, which tried again a second or three later. 500 clients connecting at once with one round trip each took 1.3–7.3 s; with the system's longest queue (decision 0079), 0.26–0.35 s. Tried and dropped (2026-11-09, Windows): connecting to an IP address without the thread, the socket's first event saying when it is done, as `mio` describes. 2,000 connections in rounds of 100, each with one round trip, took 726–982 ms against 462–576 ms with the thread, in turns. Note for a next try: before its first event, a connecting socket on Windows already gives the other side's address, so only the event tells a refused connect from a made one.

**The channel wakes one putter at a time** (2026-11-09, `woken` in `runtime/native/src/rt_channels.rs`). The profile above showed the consumer waking a producer on every take, and most producers woken found the channel full again. Now, with several workers, a channel wakes one waiting putter and no other until that one has tried again; a putter that leaves room wakes the next itself. 48 producers and one consumer on 24 workers, 10 runs each in turns: 891–1,011 → 63–82 ms, about as fast as on one core (60 ms on 2026-11-03). The 2-million pipe on one worker didn't change (90–101 ms against 92–118).

It also showed a lost wake-up in the scheduler. The last worker to go idle lets go of its lock to look for a deadlock (`all_quiet` in `runtime/core/src/tasks/pick.rs`). A task made ready in that moment found the worker awake, so its waker sent no signal, and the worker then slept with the task in its queue. Before, the next take woke another producer, which hid this; with one wake-up in flight, one run in three hung. Now the worker looks at its queue again after taking the lock back (test `a_task_made_ready_while_the_last_worker_looks_for_a_deadlock_runs`, which fails every time without it).

**Left before all cores become the default:** the test suite run with `tasks = "parallel"`.

## Risks

- Every runtime structure that assumes one thread at a time has to be found: the scheduler, channels, agents, the GPU caches (per thread already), `rt_drops.rs` (per thread already), the I/O poller.
- Tests that compare output across tasks must be written to rule (a), or run with `tasks = "coroutines"`.
- About three to four weeks of work, in the steps above.

## For a maintainer to decide

1. Rule (a), as recommended, or (b)?
2. With (a): the default `tasks` value (all cores), and keeping `"coroutines"` for one-core, repeatable runs?
