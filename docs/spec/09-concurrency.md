# 9. Concurrency

Kavach runs many lightweight **tasks** at once. There is no `async`/`await`. Any function can wait on I/O, and the runtime runs other tasks while it waits.

## 9.1 Tasks

`spawn` starts a task and returns a `Task<T>`. `.wait()` gets its result.

```kavach
fn load_dashboard(user_id: Int) -> Dashboard ! Error uses Net {
    const profile = spawn fetch_profile(user_id)
    const orders = spawn fetch_orders(user_id)

    Dashboard(
        profile: try profile.wait(),
        orders: try orders.wait(),
    )
}
```

Both fetches run at the same time.

The spawned code can fail without `try`: its error comes out of `.wait()`, which gives `T ! Error`. Above, a failed `fetch_profile` makes `try profile.wait()` pass the error on. A task's value or error stays in the `Task` until it is read; `.wait()` can be called again and gives the same.

**An error nobody reads is not lost** ([decision 0075](../decisions/0075-cancelling-tasks.md)). An error in a task never stops the program. But when a task that failed is let go of and no `.wait()` read its error, the error is printed to standard error, with where the task was spawned. That happens when the function that spawned it returns (9.2), or, for a `Task` handed out of that function, when the last holder lets it go. The program goes on, and its exit code doesn't change:

```kavach
fn risky() -> Int ! Error {
    fail Error("disk full")
}

fn main() {
    const _t = spawn risky()
    print("main goes on")
}
// main goes on
// on standard error: error in a task nobody waited for: disk full (spawned at main.kav:6)
```

The compiler keeps such tasks in sight: a `spawn` whose `Task` is thrown away is an error, like any value (3.7), and a `Task` kept in a name that is never used gets a warning (1.4). A name starting with `_`, as above, says nobody waits for it on purpose and silences the warning; an error is still printed if it happens.

A panic (5.6) stops only the task it happens in. It is printed when it happens, `panic: <message>` and its place, and `.wait()` on that task fails with an error whose message is `panicked: <message> at <file>:<line>` ([decision 0065](../decisions/0065-a-panic-stops-only-its-task.md)). A panic in the main task ends the program (5.8). A panic inside the function of a `Shared` value's `update` (6.2) leaves the value as it was, and other tasks can go on using it.

```kavach
const task = spawn [1, 2][5]
const n = task.wait() catch e => {
    print("{e}")   // panicked: index 5 is out of range: the list has 2 items at main.kav:1
    0
}
```

## 9.2 Structured: tasks never leak

A task cannot outlive the function that spawned it. When that function returns, any task still running is cancelled (9.6) and waited for. No task keeps running in the background unless you deliberately hand it to a long-lived owner such as an agent.

If one task fails and the error is passed on with `try`, its sibling tasks are cancelled.

A task that has ended is let go of while the function that spawned it still runs, so a loop that spawns a task for each request (a server) does not grow without end. A program that kept the task's handle can still `.wait()` for its result.

## 9.3 Doing many things at once

```kavach
const pages = urls.parallel_map(url => download(url), max: 10)
```

`parallel_map` runs the function on each item in a task of its own, at most `max` tasks at a time, and returns the results in the original order. Each task runs as `spawn thread` makes it (9.8), so in a release build the items compute on several cores at once. Without `max` (or with 0), every task starts at once. Before it starts the task for item `i`, it waits for the task of item `i - max`, so the turns (9.7) are the same on every run.

## 9.4 Channels

Channels pass values between tasks. `Channel<T>(capacity: n)` gives its two ends, a `Sender<T>` and a `Receiver<T>`:

```kavach
const (send, receive) = Channel<Order>(capacity: 100)

spawn {
    for order in incoming_orders() {
        send.put(order)
    }
    send.close()
}

for order in receive {
    process(order)
}
```

| Method | Does |
|---|---|
| `send.put(v)` | Adds `v`. When the channel already holds `capacity` values, it waits until one is taken. Putting into a closed channel panics. |
| `send.close()` | No more values will come. Values already in the channel can still be taken. |
| `receive.take()` | The next value, in the order they were put, as `T?`. It waits while the channel is empty, and gives `none` once the channel is closed and empty. |

`for v in receive` takes values until `take()` gives `none`. Waiting in `put` and `take` is waiting in the sense of 9.7: other tasks run meanwhile, and tasks waiting on one channel go on in the order they started waiting. If every task waits and none can go on (a receiver whose sender never closes, say), the program panics with a deadlock.

`capacity` must be at least 1. Copies of an end share the channel, like `Shared` (6.2).

## 9.5 No data races

Values are values (chapter 6), so a task always works on its own copy. The only way for tasks to change the same thing is `Shared<T>`, which locks automatically:

```kavach
const visits = Shared(0)
urls.parallel_map(url => {
    visits.update(n => n + 1)
    http.get(url)
})
```

The compiler rejects code that could create a data race. There is no "remember to lock".

## 9.6 Timeouts and cancellation

A task is **cancelled** when the function that spawned it returns while the task still runs (9.2), or by `task.cancel()`. Every kind of task can be cancelled (9.8). A task cancelled because its function returned stops at its next wait (9.7): a task that only computes finishes its work first. A task cancelled with `task.cancel()` stops at the first of these points: a wait, the end of a loop's round, or the start of a function call, so it stops even while it computes. Either way it ends there, as if every function it is in returned at once. What they hold is released, and their `Drop` cleanup runs; nothing after that point runs. The tasks a cancelled task spawned are cancelled in turn. Cancelling a task that has ended does nothing ([decision 0075](../decisions/0075-cancelling-tasks.md)).

```kavach
fn main() {
    const counting = spawn {
        let n = 0
        while true {        // never waits
            n += 1
        }
    }
    counting.cancel()       // stops it all the same
    print("stopped")
}
// stopped
```

A cancel always wins: no code can put it off. Nothing is left half-done all the same:

- Inside `Shared.update` (6.2), the task stops once the update's function returns, so the value is never half-changed.
- An agent's event that is cut short stays unfinished in its journal and runs again after a restart (8.5).
- A durable write (`fs.write_durable`, 14.3) is all there or not at all, as after a crash.

**A time limit at spawn** stops a task that may get stuck, even one that holds the turn (9.7) so nothing else can run to cancel it. `within d` after the spawned call or block gives it `d` to finish; then it stops at its next such point, as if cancelled, and `.wait()` on it fails with `timed out after 5000 ms`:

```kavach
const result = spawn search(board) within 5.seconds
const best = result.wait() catch e => {
    print(e)        // timed out after 5000 ms
    fallback
}
```

Only a program that calls `cancel()` or gives a time limit somewhere gets these checks; others pay nothing. A counted loop whose body runs straight through (no call and no inner loop) gets none: it ends anyway, and it stays a loop the compiler can vectorize or run on all cores.

```kavach
use std.time

fn watch() {
    const ticker = spawn {
        for tick in time.every_ms(100) {
            print("tick {tick}")
        }
    }
    time.sleep_ms(350)
}   // `ticker` is cancelled here: it stops at its next round

fn main() {
    watch()
    print("watching stopped")
}
// tick 0
// tick 1
// tick 2
// watching stopped
```

Cancelling is not an error: `catch` and `match` don't see it, so a task can't refuse to stop. `.wait()` on a cancelled task (a `Task` handed out of the function that spawned it) fails with the error `the task was cancelled`.

A wait can be given a time limit, a `Duration` (14.4), with `.timeout(d)` on `task.wait()`, `receive.take()`, `send.put(v)` and the network waits of [14.10](14-standard-library/10-net.md#1410-stdnet). It waits at most `d`; then it stops waiting and fails with an error, `timed out after 5000 ms` (a network wait fails with `NetError.TimedOut`). A `.wait()` that timed out leaves the task running (it is still cancelled when its function returns); a `put` that timed out puts nothing.

```kavach
const page = try fetch.wait().timeout(5.seconds)
const next = receive.take().timeout(200.ms) catch e => none
```

## 9.7 Taking turns

Tasks take turns in an order the language defines, so a program prints the same every time it runs, in development and in a release build:

- `spawn` makes the new task ready. The task that spawned it keeps running.
- A task runs until it **waits**: for another task (`.wait()`, or a function that spawned tasks returning, 9.2), for time (`time.sleep_ms`, a round of `time.every_ms`), and later for channels and I/O. Then the task that became ready first runs.
- A waiting task becomes ready when what it waits for happens. When no task is ready, the program waits for the earliest of them.

```kavach
use std.time

fn report(name: String, ms: Int) -> String {
    time.sleep_ms(ms)
    print("{name} done")
    name
}

fn main() ! Error {
    const slow = spawn report("slow", 200)
    const fast = spawn report("fast", 100)
    print("both started")
    print("{try slow.wait()} and {try fast.wait()} are back")
}
// both started
// fast done
// slow done
// slow and fast are back
```

The two sleeps overlap, so `main` takes about 200 ms, not 300.

Turns fix the order of what tasks do that others can see, not when they compute. For a task spawned with `spawn thread` (9.8), a release build (`kavach build`) lets a new task compute ahead, on another core, until its first such effect (printing, files, channels, spawning, waiting, ending), which then waits for its turn ([decision 0020](../decisions/0020-tasks-take-turns.md)). Values are copied into a task (9.5), so this changes no program's results, only how fast it runs.

## 9.8 How tasks run

Each `spawn` says how its task runs. With no word, the task is a coroutine ([decision 0069](../decisions/0069-each-task-chooses-how-it-runs.md)):

```kavach
spawn handle(conn)            // a coroutine: the default
spawn thread read_sensor()    // a thread of its own
spawn cores render(frame)     // on any core of the machine
```

| Written | How the task runs | Best for |
|---|---|---|
| `spawn` | A coroutine on the thread of the task that spawned it: passing the turn needs no operating system ([decision 0048](../decisions/0048-tasks-as-coroutines-per-core.md)) | Many tasks that mostly wait, such as a server's task per connection |
| `spawn thread` | An operating-system thread of its own; a release build (`kavach build`) lets it compute ahead on another core until its first effect (9.7) | A task that blocks inside a C library, or computes for long |
| `spawn cores` | On a worker thread per core, the next one in turn, started when the first such task is spawned | Heavy computing, spread over every core |
| `spawn process` | An operating-system process of its own, running the program's code; the call's arguments and its result are copied in and out | Code Kavach can't check, such as a C or Python library that may hang: `cancel()` ends the process at once |

- `spawn`, `spawn thread` and `spawn cores` take the same things: a call or a block, and the task is a `Task<T>` either way.
- `thread`, `cores` and `process` are words only right after `spawn`; anywhere else they are names like any other.
- `spawn process` takes a call of a function, not a block. Its arguments and its result must be values that can be copied out of the program: no `Shared` values, channels, tasks, functions or agents; the compiler says which one can't. What the task prints goes to the program's output, in turn like any task's. Cancelling it ends the process at once: it shares nothing, so nothing is left half-changed ([decision 0075](../decisions/0075-cancelling-tasks.md)). The function's error comes out of `.wait()`, as any task's does, and so does a panic in it, as `panicked: <message> at <file>:<line>` (9.1). Starting a process takes milliseconds, so it is for long or risky work. A thread is never ended this way: one stopped in the middle of anything can leave what it shares half-changed.
- Every kind takes turns as 9.7 says, so whichever is chosen, a program prints the same: only its speed changes. A `spawn cores` task keeps a looser order, the one of [decision 0049](../decisions/0049-tasks-on-all-cores-keep-the-programs-order.md): its own lines come out in the order it prints them, whole, and a line printed before a task sends a value, spawns a task or ends comes before what the receiver prints after that. Channels and `Shared` values work across cores: a value going through either is copied for the task that gets it, and `update` holds its value's lock (6.2).
- Only a release build runs `spawn cores` tasks on every core today. Other runs (`kavach run`, `run --native`) run them on one core, in turn, and print the same.
- How tasks run is no setting of `kavach.toml`: `tasks =` there is an error that says to choose in the code.

## Open questions

- `select` over several channels: syntax and whether it is needed in v1.
- `spawn process` of a generic function, and `Bytes` going in or out of one: both are refused for now, since the arguments and the result go between the processes as JSON, and `Bytes` has no JSON form yet.
- A time limit on any call that waits inside (`download(url).timeout(5.seconds)`): cancel the call's work when the time passes, like a task's.
- Closing a channel when its last `Sender` is gone, instead of with `close()`: when that happens differs between the interpreter and native code, which frees a variable after its last use.
