# 9. Concurrency

Conformance tests for spec chapter [9. Concurrency](../../../docs/spec/09-concurrency.md). Format: [../README.md](../README.md). Tasks take turns ([decision 0020](../../../docs/decisions/0020-tasks-take-turns.md)); in programs built by `kavach build`, new tasks compute ahead on all cores; a function's tasks are cancelled when it returns.

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 9.1 | [spawn-and-wait.kav](spawn-and-wait.kav) | `spawn` starts a task and `.wait()` gives its result. | runs |
| 9.1 | [wait-gives-the-task-error.kav](wait-gives-the-task-error.kav) | Spawned code may fail without `try`: `.wait()` gives its value, or its error. | runs |
| 9.1 | [a-panic-stops-only-its-task.kav](a-panic-stops-only-its-task.kav) | A panic stops only its task: printed when it happens, `.wait()` fails with `panicked: … at …`, and the program goes on. | runs |
| 9.1 | [a-panic-inside-update-lets-the-value-go.kav](a-panic-inside-update-lets-the-value-go.kav) | A panic inside a `Shared` value's `update` leaves the value as it was, and other tasks can go on using it. | runs |
| 9.2 | [functions-wait-for-their-tasks.kav](functions-wait-for-their-tasks.kav) | When a function returns, its tasks are cancelled and waited for. | runs |
| 9.2 | [ended-tasks-are-let-go.kav](ended-tasks-are-let-go.kav) | A task that has ended is let go of while its function runs; a handle the program kept still gives the result. | runs |
| 9.7 | [tasks-take-turns.kav](tasks-take-turns.kav) | A spawned task runs when the running one waits; ready tasks run in the order they became ready. | runs |
| 9.7 | [sleeping-tasks-overlap.kav](sleeping-tasks-overlap.kav) | Other tasks run while one sleeps, so sleeps overlap; tasks wake in time order. | runs |
| 9.4 | [channels-pass-values-in-order.kav](channels-pass-values-in-order.kav) | A channel passes values in order; `put` waits while it is full, `take` while it is empty; a closed, empty channel gives `none`. | runs |
| 9.4 | [a-for-over-a-channel-takes-values-of-every-kind.kav](a-for-over-a-channel-takes-values-of-every-kind.kav) | `for v in receive` takes every value until the channel is closed and empty, whatever the values' type. | runs |
| 9.4 | [a-receiver-nobody-closes-deadlocks.kav](a-receiver-nobody-closes-deadlocks.kav) | When every task waits and none can go on, the program panics with a deadlock. | panic |
| 9.4 | [put-into-a-closed-channel-panics.kav](put-into-a-closed-channel-panics.kav) | Putting a value into a closed channel panics. | panic |
| 9.4 | [channel-capacity-is-at-least-one.kav](channel-capacity-is-at-least-one.kav) | A channel's capacity must be at least 1. | panic |
| 9.4 | [try-put-never-waits.kav](try-put-never-waits.kav) | `send.try_put(v)` adds `v` only when the channel has room and is open, and says whether it did; it never waits. | runs |
| 9.4 | [a-sent-none-is-not-the-end.kav](a-sent-none-is-not-the-end.kav) | `take()` gives `Taken.Item(value)` or `Taken.Closed`, so a `none` sent on a `Channel<T?>` is a value, not the channel's end. | runs |
| 9.3 | [parallel-map-keeps-order-and-limit.kav](parallel-map-keeps-order-and-limit.kav) | `parallel_map` runs each item in its own task, at most `max` at a time, and gives the results in order. | runs |
| 9.3 | [parallel-map-passes-a-panic-on.kav](parallel-map-passes-a-panic-on.kav) | A panic in an item's task of `parallel_map` is a panic where `parallel_map` was called. | panics |
| 9.7 | [effects-keep-the-order-of-turns.kav](effects-keep-the-order-of-turns.kav) | Tasks may compute at the same time, but what they print happens in the order of their turns. | runs |
| 9.8 | [each-spawn-chooses-how-it-runs.kav](each-spawn-chooses-how-it-runs.kav) | `spawn`, `spawn thread` and `spawn cores`, mixed in one program, print the same. | runs |
| 9.5 | [tasks-share-a-shared-value.kav](tasks-share-a-shared-value.kav) | Tasks that hold a `Shared` value change that one value, in the order of their turns. | runs |
| 9.6 | [cancelled-tasks-stop-at-their-next-wait.kav](cancelled-tasks-stop-at-their-next-wait.kav) | A cancelled task stops at its next waiting point; nothing after it runs. | runs |
| 9.6 | [cancel-stops-a-task-that-never-waits.kav](cancel-stops-a-task-that-never-waits.kav) | `task.cancel()` stops a task at once, even in a loop that never waits. | runs |
| 9.6 | [cancelled-tasks-run-their-cleanup.kav](cancelled-tasks-run-their-cleanup.kav) | A task's cleanup is a `Drop` value: it runs when the task is cancelled while it waits. | runs |
| 9.6 | [cancel-keeps-errors-of-calls.kav](cancel-keeps-errors-of-calls.kav) | In a program that cancels tasks, a call that fails still fails where it is made, also when its arguments are calls. | runs |
| 9.6 | [a-time-limit-stops-a-stuck-task.kav](a-time-limit-stops-a-stuck-task.kav) | `spawn … within d` stops a stuck task when its time is up; `.wait()` fails with `timed out after … ms`. | runs |
| 9.6 | [cancelling-is-not-an-error.kav](cancelling-is-not-an-error.kav) | `catch` doesn't see cancelling, so a cancelled task stops even inside `catch`. | runs |
| 9.6 | [waiting-for-a-cancelled-task-fails.kav](waiting-for-a-cancelled-task-fails.kav) | `.wait()` on a cancelled task fails with the error `the task was cancelled`. | runs |
| 9.6 | [timeouts-end-a-wait.kav](timeouts-end-a-wait.kav) | `.timeout(d)` on `wait`, `take` and `put` waits at most `d`, then fails with an error. | runs |
| 9.1 | [errors-nobody-reads-are-printed.kav](errors-nobody-reads-are-printed.kav) | A task's error that no `.wait()` read is printed to standard error, with where the task was spawned, and the program goes on. | runs |
| 9.1 | [errors-that-were-read-are-not-printed.kav](errors-that-were-read-are-not-printed.kav) | A task's error that `.wait()` read is not printed again. | runs |
| 9.8 | [spawn-process-runs-a-call-in-its-own-process.kav](spawn-process-runs-a-call-in-its-own-process.kav) | `spawn process f(args)` runs the call in a process of its own: arguments in as copies, its value back, its prints in turn. | runs |
| 9.8 | [a-process-error-comes-out-of-wait.kav](a-process-error-comes-out-of-wait.kav) | The error of a `spawn process` call comes out of `.wait()`. | runs |
| 9.8, 9.1 | [a-process-panic-fails-its-task.kav](a-process-panic-fails-its-task.kav) | A panic in a `spawn process` call stops only its task: `.wait()` fails with `panicked: … at …`. | runs |
| 9.8 | [cancel-ends-a-process.kav](cancel-ends-a-process.kav) | `cancel()` ends a `spawn process` task's process at once, even one stuck in a loop. | runs |
| 9.8 | [spawn-process-takes-a-call.kav](spawn-process-takes-a-call.kav) | `spawn process` takes a call of a function by its name, not a block. | compile error |
| 9.8 | [a-printed-line-comes-out-whole.kav](a-printed-line-comes-out-whole.kav) | A printed line comes out whole: nothing of it is written while its values wait. | runs |
