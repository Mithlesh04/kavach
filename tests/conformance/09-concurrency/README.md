# 9. Concurrency

Conformance tests for spec chapter [9. Concurrency](../../../docs/spec/09-concurrency.md). Format: [../README.md](../README.md). Tasks take turns ([decision 0020](../../../docs/decisions/0020-tasks-take-turns.md)); in programs built by `kavach build`, new tasks compute ahead on all cores; a function's tasks are cancelled when it returns.

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 9.1 | [spawn-and-wait.kav](spawn-and-wait.kav) | `spawn` starts a task and `.wait()` gives its result. | runs |
| 9.1 | [a-spawn-alone-is-waited-for-by-nobody.kav](a-spawn-alone-is-waited-for-by-nobody.kav) | A `spawn` on a line of its own starts a task nobody waits for; an error in it is printed. | runs |
| 9.1 | [wait-gives-the-task-error.kav](wait-gives-the-task-error.kav) | Spawned code may fail without `try`: `.wait()` gives its value, or its error. | runs |
| 9.1 | [a-panic-stops-only-its-task.kav](a-panic-stops-only-its-task.kav) | A panic stops only its task: printed when it happens, `.wait()` fails with `panicked: … at …`, and the program goes on. | runs |
| 9.1 | [a-panic-inside-update-lets-the-value-go.kav](a-panic-inside-update-lets-the-value-go.kav) | A panic inside a `Shared` value's `update` leaves the value as it was, and other tasks can go on using it. | runs |
| 9.1, 6.2 | [a-panic-in-one-update-loses-no-other-tasks-update.kav](a-panic-in-one-update-loses-no-other-tasks-update.kav) | A panic inside one task's `update`, while tasks on every core update the same `Shared` value, lets go only of that task's hold on it: no other task's update is lost. | runs |
| 9.1 | [a-spawned-call-gets-its-arguments-worked-out-first.kav](a-spawned-call-gets-its-arguments-worked-out-first.kav) | A spawned call's arguments, and the value a method is called on, are worked out at the spawn, in the order written; the task makes the call with those values. A spawned block runs whole in the task. | runs |
| 9.1 | [a-spawned-block-reads-fields-as-they-were-at-the-spawn.kav](a-spawned-block-reads-fields-as-they-were-at-the-spawn.kav) | A spawned block that reads only some fields of a record sees them as they were at the spawn: the caller changing the record afterwards doesn't change what the task sees. | runs |
| 9.1 | [a-panic-in-a-spawned-argument-happens-at-the-spawn.kav](a-panic-in-a-spawned-argument-happens-at-the-spawn.kav) | A panic while a spawned call's arguments are worked out happens at the spawn, in the spawning code, and no task starts. | panic |
| 9.1 | [a-try-in-a-spawned-argument-is-the-spawners.kav](a-try-in-a-spawned-argument-is-the-spawners.kav) | A `try` in a spawned call's argument passes the error on from the spawning function, before any task starts. | runs |
| 9.1, 2.7 | [a-spawned-call-through-optional-runs-in-the-task.kav](a-spawned-call-through-optional-runs-in-the-task.kav) | A spawned call through `?.` runs whole in the task: its arguments are worked out there, and only when there is a value. | runs |
| 9.1 | [a-spawned-function-is-worked-out-at-the-spawn.kav](a-spawned-function-is-worked-out-at-the-spawn.kav) | A spawned call whose function is worked out from an expression works it out at the spawn, before the arguments, as Go does. | runs |
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
| 9.4 | [a-channel-with-no-receivers-takes-nothing.kav](a-channel-with-no-receivers-takes-nothing.kav) | Once every receiver is gone, `try_put` gives `false` and `put` panics. | panics |
| 9.4 | [a-waiting-put-ends-when-the-receivers-go.kav](a-waiting-put-ends-when-the-receivers-go.kav) | A `put` waiting for room panics when the last receiver goes. | panics |
| 9.3 | [parallel-map-keeps-order-and-limit.kav](parallel-map-keeps-order-and-limit.kav) | `parallel_map` runs each item in its own task, at most `max` at a time, and gives the results in order. | runs |
| 9.3 | [parallel-map-passes-a-panic-on.kav](parallel-map-passes-a-panic-on.kav) | A panic in an item's task of `parallel_map` is a panic where `parallel_map` was called. | panics |
| 9.7 | [effects-keep-the-order-of-turns.kav](effects-keep-the-order-of-turns.kav) | Tasks may compute at the same time, but what they print happens in the order of their turns. | runs |
| 9.8 | [each-spawn-chooses-how-it-runs.kav](each-spawn-chooses-how-it-runs.kav) | `spawn`, `spawn thread` and `spawn cores`, mixed in one program, print the same. | runs |
| 9.5 | [tasks-share-a-shared-value.kav](tasks-share-a-shared-value.kav) | Tasks that hold a `Shared` value change that one value, in the order of their turns. | runs |
| 9.6 | [cancelled-tasks-stop-at-their-next-wait.kav](cancelled-tasks-stop-at-their-next-wait.kav) | A cancelled task stops at its next waiting point; nothing after it runs. | runs |
| 9.6 | [cancel-stops-a-task-that-never-waits.kav](cancel-stops-a-task-that-never-waits.kav) | `task.cancel()` stops a task at once, even in a loop that never waits. | runs |
| 9.6 | [cancelled-tasks-run-their-cleanup.kav](cancelled-tasks-run-their-cleanup.kav) | A task's cleanup is a `Drop` value: it runs when the task is cancelled while it waits. | runs |
| 9.6 | [cancelled-tasks-drop-at-once.kav](cancelled-tasks-drop-at-once.kav) | A task cancelled while it waits runs its `drop`s as it stops, not when another task next runs. | runs |
| 9.6 | [cancel-keeps-errors-of-calls.kav](cancel-keeps-errors-of-calls.kav) | In a program that cancels tasks, a call that fails still fails where it is made, also when its arguments are calls. | runs |
| 9.6 | [a-time-limit-stops-a-stuck-task.kav](a-time-limit-stops-a-stuck-task.kav) | `spawn … within d` stops a stuck task when its time is up; `.wait()` fails with `timed out after … ms`. | runs |
| 9.6 | [cancelling-is-not-an-error.kav](cancelling-is-not-an-error.kav) | `catch` doesn't see cancelling, so a cancelled task stops even inside `catch`. | runs |
| 9.6 | [waiting-for-a-cancelled-task-fails.kav](waiting-for-a-cancelled-task-fails.kav) | `.wait()` on a cancelled task fails with the error `the task was cancelled`. | runs |
| 9.6 | [timeouts-end-a-wait.kav](timeouts-end-a-wait.kav) | `.timeout(d)` on `wait`, `take` and `put` waits at most `d`, then fails with an error. | runs |
| 9.1 | [errors-nobody-reads-are-printed.kav](errors-nobody-reads-are-printed.kav) | A task's error that no `.wait()` read is printed to standard error, with where the task was spawned, and the program goes on. | runs |
| 9.1 | [errors-that-were-read-are-not-printed.kav](errors-that-were-read-are-not-printed.kav) | A task's error that `.wait()` read is not printed again. | runs |
| 9.1 | [a-task-panic-is-printed-once.kav](a-task-panic-is-printed-once.kav) | A task's panic passed on by `main` is printed once; the last line says the panic above ended the program. | runs |
| 9.8 | [spawn-process-runs-a-call-in-its-own-process.kav](spawn-process-runs-a-call-in-its-own-process.kav) | `spawn process f(args)` runs the call in a process of its own: arguments in as copies, its value back, its prints in turn. | runs |
| 9.8 | [spawn-process-passes-bytes.kav](spawn-process-passes-bytes.kav) | `Bytes` go into and out of a `spawn process` call, as base64 text in the JSON between the processes. | runs |
| 9.8 | [a-process-error-comes-out-of-wait.kav](a-process-error-comes-out-of-wait.kav) | The error of a `spawn process` call comes out of `.wait()`. | runs |
| 9.8, 9.1 | [a-process-panic-fails-its-task.kav](a-process-panic-fails-its-task.kav) | A panic in a `spawn process` call stops only its task: `.wait()` fails with `panicked: … at …`. | runs |
| 9.8 | [cancel-ends-a-process.kav](cancel-ends-a-process.kav) | `cancel()` ends a `spawn process` task's process at once, even one stuck in a loop. | runs |
| 9.8 | [spawn-process-takes-a-call.kav](spawn-process-takes-a-call.kav) | `spawn process` takes a call of a function by its name, not a block. | compile error |
| 9.8 | [a-printed-line-comes-out-whole.kav](a-printed-line-comes-out-whole.kav) | A printed line comes out whole: nothing of it is written while its values wait. | runs |
| 1.4, 9.4 | [a-const-channel-end-cannot-be-replaced.kav](a-const-channel-end-cannot-be-replaced.kav) | a channel end named with `const` can't be given another end; use `let` for one that changes. | compile error |
| 1.4, 9.5 | [a-name-in-a-spawn-block-hides-the-outer-one.kav](a-name-in-a-spawn-block-hides-the-outer-one.kav) | a spawn block is an inner scope: it may declare a name the outside already has, and the outside name stays as it was. | runs |
| 1.4, 9.5 | [an-unused-hiding-let-in-a-spawn-block-warns.kav](an-unused-hiding-let-in-a-spawn-block-warns.kav) | a never-used `let` inside a spawn block that hides an outside `let` of the same name gets the hiding warning. | warning |
| 1.4, 9.4 | [unused-channel-names-warn.kav](unused-channel-names-warn.kav) | a channel end that a pattern names but nothing uses, and a `for` name over a channel that the body never reads, get the unused-name warning; `_` names get none. | warning |
| 3.8, 5.6, 9.1 | [a-stack-overflow-stops-only-its-task.kav](a-stack-overflow-stops-only-its-task.kav) | Running out of stack in a task is a panic of that task, however the recursion goes: `.wait()` fails with `panicked: …`, and the program goes on. | runs |
| 5.6, 9.1 | [an-out-of-memory-panic-stops-only-its-task.kav](an-out-of-memory-panic-stops-only-its-task.kav) | Running out of memory in a task is a panic of that task: `.wait()` fails with `panicked: …`, and the program goes on. | runs |
| 9.1, 1.4 | [an-unused-task-name-warns.kav](an-unused-task-name-warns.kav) | a `Task` kept in a name that is never used gets the unused-name warning; a name starting with `_` says nobody waits on purpose and gets none. | warning |
| 9.1, 9.5 | [waiting-again-gives-the-same-value.kav](waiting-again-gives-the-same-value.kav) | a task's value stays in its `Task` and `.wait()` gives the same every time; the caller's copy is its own, so changing it changes neither the task's value nor later waits. | runs |
| 9.4, 9.7 | [a-capacity-one-channel-holds-one-value.kav](a-capacity-one-channel-holds-one-value.kav) | a channel of capacity 1 holds one value: a second `put` waits until it is taken, so a producer and a taker alternate. | runs |
| 9.4 | [a-closed-channel-is-closed-for-every-copy.kav](a-closed-channel-is-closed-for-every-copy.kav) | a channel closed through one copy of its sender is closed for every copy: `try_put` gives `false`, and `put` panics. | panic |
| 9.4, 9.5 | [a-sent-value-is-a-copy.kav](a-sent-value-is-a-copy.kav) | a value put into a channel is a copy: changing the variable after `put` doesn't change what the receiver takes. | runs |
| 9.4, 6.2 | [a-shared-sent-through-a-channel-is-the-same-value.kav](a-shared-sent-through-a-channel-is-the-same-value.kav) | a `Shared` put into a channel is a copy of the link, so the taker updates the value the sender sees. | runs |
| 9.4, 4.3, 4.4 | [a-taken-value-is-taken-apart-by-patterns.kav](a-taken-value-is-taken-apart-by-patterns.kav) | a value taken from a channel can be taken apart: a `for` over a channel of tuples names their parts, and a `match` on `Taken.Item` takes the tuple apart. | runs |
| 9.4 | [channel-end-copies-share-the-channel.kav](channel-end-copies-share-the-channel.kav) | copies of a channel end share the channel, wherever they are kept: a `let`, a parameter, a record field, a list, a lambda or a task. | runs |
| 9.4 | [channels-carry-unions-optionals-and-tuples.kav](channels-carry-unions-optionals-and-tuples.kav) | a channel carries values of any type whole: unions with their fields, optionals, tuples, lists of records. | runs |
| 9.5, 3.3 | [a-task-gets-a-copy-of-what-it-uses.kav](a-task-gets-a-copy-of-what-it-uses.kav) | a task works on its own copy of the outside values it uses, taken when it is spawned: the caller changing a `let` afterwards doesn't change what the task sees, and a `const` is read the same way. | runs |
| 9.5, 3.3 | [a-task-that-changes-a-captured-let-warns.kav](a-task-that-changes-a-captured-let-warns.kav) | a task that changes an outside `let`, with `+=` or a changing method such as `push`, changes only its own copy; the compiler warns, and the caller's variable stays as it was. | warning |
| 9.5, 4.4 | [each-spawn-in-a-loop-copies-that-rounds-values.kav](each-spawn-in-a-loop-copies-that-rounds-values.kav) | a task spawned in a loop copies the values it uses in that round: the loop's name and a `let` the loop goes on changing. | runs |
| 9.5, 3.3 | [spawn-takes-its-arguments-when-it-starts.kav](spawn-takes-its-arguments-when-it-starts.kav) | `spawn f(x)` copies its arguments when the task is spawned, and a lambda it calls keeps the copies it took when made: later changes by the caller don't reach the task. | runs |
| 9.8 | [spawn-process-refuses-a-channel-end.kav](spawn-process-refuses-a-channel-end.kav) | a `spawn process` call can't take a channel end: what it takes is copied out of the program, and a channel can't be. | compile error |
| 9.8 | [spawn-process-refuses-a-shared.kav](spawn-process-refuses-a-shared.kav) | a `spawn process` call can't take a `Shared` value: what it takes is copied out of the program, and a `Shared` can't be. | compile error |
