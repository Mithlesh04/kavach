# 0075. Cancelling tasks, stuck ones too

- **Status:** Accepted (2026-11-06, Mithlesh chose the recommendation on all four questions)
- **Date:** 2026-11-06
- **Changes:** spec 9.1, 9.2, 9.6, 9.8; the checker; both runtimes

## Context

Mithlesh asked (2026-11-06): can a program cancel a task it spawned, whatever its kind (`spawn`, `spawn thread`, `spawn cores`)? When a cancel is asked for, the task must stop even when it is stuck, for example in a loop that never ends. And what happens to errors in tasks: can one bring the whole program down? Some of this may need an `unsafe` way.

What Kavach does today (spec 9):

- **A task is cancelled only when the function that spawned it returns** (9.2). There is no `task.cancel()`.
- **A cancelled task stops at its next waiting point** (9.6): a sleep, a channel, the network, `.wait()`. A task that computes goes on until it waits or ends. A loop that never waits can't be stopped at all.
- **An error in a task never stops the program.** It stays in the `Task` and comes out of `.wait()` (9.1). But an error nobody reads is lost without a word: `const _t = spawn risky()` then `main` returning prints nothing, and the program exits with 0.
- **A panic stops only its task** ([decision 0065](0065-a-panic-stops-only-its-task.md)). Until native code can unwind, a panic still ends the program.

## What others do

| Language | How a task is stopped | A stuck loop | Forced kill |
|---|---|---|---|
| Go | No cancel: a `context` the code checks | The runtime interrupts tight loops with signals since 1.14 ([async preemption](https://golangweekly.com/link/87958/web)), only to switch goroutines, not to stop them | None |
| Java | `interrupt()`, which the code checks | Not stopped | `Thread.stop` throws since JDK 20 and is being removed: it corrupted shared state ([JDK-8293852](https://bugs.openjdk.org/browse/JDK-8293852), [an example](https://4comprehension.com/how-to-stop-a-java-thread-without-using-thread-stop/)) |
| .NET | `CancellationToken`, checked by the code | Not stopped | `Thread.Abort` is gone since .NET 5, for the same reason |
| Kotlin, Swift | `job.cancel()`, `task.cancel()`: cooperative, at suspension points; a loop calls `ensureActive()` ([Kotlin](https://kotlinlang.org/api/kotlinx.coroutines/kotlinx-coroutines-core/kotlinx.coroutines/ensure-active.html)) | Only if the loop checks | None |
| Rust (Tokio) | `handle.abort()`: stops at the next `.await` | Not stopped | None |
| Erlang | `exit(Pid, kill)`: can't be caught, stops the process at once ([docs](https://build.opensuse.org/projects/home:Ledest:erlang:20/packages/erlang/files/1153-erlang-module-doc-more-accurate-description-of-exit-.patch?expand=0)) | Stopped: the scheduler counts work and checks often | Safe, because processes share no memory |
| JavaScript | `worker.terminate()` | Stopped | Safe, because a worker shares no memory |
| Python (asyncio) | `task.cancel()` at the next `await` | Not stopped | None; an error nobody reads is printed, "Task exception was never retrieved" |

The lesson: **a forced kill is safe only when the task shares nothing it could leave half-changed.** Java and .NET removed theirs. Erlang and JavaScript keep theirs, because their tasks are isolated. Kavach tasks already work on copies (9.5): only `Shared` values, channels, files and agents are shared, and each of those can be made safe at the moment a task stops.

## Proposal

1. **`task.cancel()`**, for every kind of task. The task stops; the tasks it spawned are cancelled too; `.wait()` then fails with `the task was cancelled`, as today. Cancelling a task that has ended does nothing. Whoever holds the `Task` may cancel it.
2. **A cancelled task stops even when it computes.** Besides the waiting points, the compiler adds a check where code can run for a long time without waiting: at the end of each loop's round and at the start of each function, so recursion is covered too. A check reads one flag the core keeps in its cache. A loop whose rounds are few and known (a counted loop over a small range, a loop the compiler vectorizes) gets none. The cost is measured on the benchmarks; it must stay under 2%. Go's way, interrupting the thread with a signal, costs nothing in the loop, so it may replace the checks later, with the same meaning.
3. **What stopping does.** Every function the task is in returns at once: what they hold is released and their `Drop` cleanup runs (9.6), as now. Shared things stay whole:
   - Inside `Shared.update`, the stop waits until the update's function returns, so a value is never left half-changed.
   - An agent's event that is cut short stays unfinished in its journal and runs again after a restart (8.5).
   - A file being written is closed; a durable write (`fs.write_durable`) is either all there or not at all, as after a crash.
4. **Code Kavach can't check: `spawn process`.** C or Python code that hangs never reaches a check. A task started with `spawn process` runs in an operating-system process of its own. Values go in and out as copies (they already do, 9.5), so `task.cancel()` on it can end the process at once, safely, as Erlang and JavaScript do.
5. **No forced kill of a thread.** Stopping a thread in the middle of anything is what Java and .NET removed. Points 2 and 4 cover every case without it.
6. **An error nobody reads is never lost.** When a task that failed is let go of without its error having been read (its `Task` goes, or its function returns), the error is printed to standard error with where the task was spawned: `error in a task nobody waited for: disk full (spawned at main.kav:6)`. The program goes on; its exit code doesn't change. Python's asyncio does the same. A `Task` value that is never waited for, of a task that can fail, also gets a compiler warning.
7. **A panic stops only its task** (decision 0065), once native code can unwind (that decision's steps 2 and 3, already on the to-do list).

## Mithlesh's choices (2026-11-06)

1. Stuck loops: checks the compiler adds (not signals, not only waits).
2. Code Kavach can't check: `spawn process` (no forced kill of a thread, not even with `unsafe`).
3. An error nobody reads: printed, and the program goes on (not failing the spawning function, not a warning only).
4. Regions a cancel waits for: none; a cancel always wins, and cleanup goes in `Drop` (no `protect { }` block).

## Open question (found while building)

A task is cancelled when the function that spawned it returns (9.2). Until 0075, a cancel stopped a task only at a wait, so a task that only computes still finished its work (`spawn { print(sum_to(1_000_000)) }` at the end of `main` prints). With 0075's checks in loops and calls, should that cancel stop computing tasks too, so a function that doesn't `.wait()` for a task cuts its work off? Or should only `task.cancel()` stop a computing task, while a function's end stops its tasks at their waits, as before? (Asked of Mithlesh, 2026-11-06.)

Recommendation: only `task.cancel()` stops computing tasks; a function's end keeps today's rule. Code that spawns work and returns keeps working, and servers' waiting tasks still stop when their function returns.

**A second question, found while building (2026-11-06).** Tasks take turns (9.7): a task hands the turn on only when it waits. A task stuck in a loop that never waits keeps the turn, so the code that would call `task.cancel()` never runs again: the whole program is stuck, not only the task. The checks in loops stop such a task only when the canceller gets to run: the task hadn't started yet, it runs ahead on another core (`spawn thread` and `spawn cores` in a release build), or it gave up the turn once and computes again. Options:

1. **A time limit given when the task starts**, checked by the task itself at its loop checks: `spawn work() within 5.seconds`. A stuck task stops itself when its time is up, so nobody needs the turn to stop it. The checks read the clock only now and then, so they stay cheap. (Recommended.)
2. **Taking the turn away from a task that computes too long**, as Go's preemption does. Then `cancel()` can always be called. But when a task's turn ends would depend on time, so the order of what tasks print could differ from run to run, which 9.7 promises it won't.
3. **Nothing more**: `cancel()` stops computing tasks only in the cases above, and a task stuck while it holds the turn stops the program.

**Mithlesh's answers (2026-11-06)**, both the recommendations:

- **A function's end keeps today's rule.** Its tasks stop at their next wait; a task that only computes finishes its work. Only `task.cancel()` stops a task at loops and calls too.
- **A time limit at spawn stops a stuck task.** `spawn work() within 5.seconds`: the task checks its own deadline at the same points (waits, loop rounds, calls) and stops itself when its time is up, as if cancelled. It needs nobody else to get the turn, and the order of what tasks print stays the same on every run. `.wait()` on it fails with `timed out after 5000 ms`, as other time limits do (9.6).

## Why

- **One word, every kind of task.** `cancel` already means what 9.6 says; it only gains a way to be asked for.
- **"Stops at any cost" without corrupting anything.** Checks inside loops stop Kavach code anywhere. Values aren't shared, and the few shared things (`Shared`, journals, durable files) are made whole at the stop. So nothing has to be killed blindly.
- **What is isolated may be killed.** A process shares no memory, so ending it is safe; a thread is not, so it is never killed.
- **Errors aren't lost.** A failed task nobody waits for is the quiet bug of Go goroutines and JavaScript promises; printing it costs nothing when nothing fails.
