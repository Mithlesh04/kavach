# Crashes, restarts and clock changes

What a Kavach program does when it doesn't end normally, or when time doesn't run as expected: the program crashes or is killed, the machine restarts, loses power or sleeps, or someone changes its clock or time zone. For each case: what happens today, and its status. Asked for by Mithlesh (2026-11-04): "create all the cases around that, it's critical".

Status words: **Handled** (works as it should), **Fixed** (was wrong, fixed with a test), **To do** (wrong or missing; in the [to-do](../vision/todo.md)), **Maintainer** (a choice for a maintainer, listed at the end).

## How Kavach keeps time

- **Waits are measured on a clock that never jumps** (Rust's `Instant`): `time.sleep`, `every`, `after`, `.timeout(d)` (spec 9.6), network deadlines, `std.ai` timeouts and retries, and limit windows (8.4). Changing the wall clock doesn't move them.
- **Moments are UTC**: `time.now_ms()`, `today()`, `now()`, audit log and OTLP times. Kavach never reads the machine's time zone, so changing it changes nothing.
- **Nothing about a wait is saved.** A wait lasts only while the program runs.

## Time

| Case | What happens today | Status |
|---|---|---|
| The wall clock is changed while the program runs | Waits, timeouts and limit windows don't move. `now_ms` jumps with the clock, so audit lines can be out of order | Handled |
| The machine's time zone is changed | Nothing: Kavach never reads it | Handled |
| The program restarts during `sleep(3.minutes)` in a durable handler | The journal has when the wait ends, so the handler waits only for what is left, never more than 3 minutes (a clock set back can't stretch it). The same for each round of `every` and for `.timeout(d)` | Fixed |
| A durable handler reads `time.now_ms()` (or `today()`, `now()`) | The time is journaled, so a run after a crash sees the same time and makes the same calls. Inside a capability's implementation it isn't: the call's outcome is | Fixed |
| The machine sleeps during a wait (a laptop's lid closed) | On Windows the wait has passed when it wakes; on Linux and macOS the clock stops while asleep, so every wait and window lasts longer by the sleep. This is the system's own clock, as in Rust, Go, Java and Python (spec 14.4) | Handled (decision 0062) |
| Limit windows after the machine restarted | Saved calls older than the machine's uptime were dropped, so a `per day` limit forgot them and loosened | Fixed |
| The clock was set back while the program was down | A saved call that seems still to come counts as made now: the limit can only get stricter | Fixed (spec 8.4) |
| The clock was set forward while the program was down | Saved calls look older and leave their window early: the limit loosens. Nothing on the machine can tell a jump from real time passing, and spec 8.4 says so | Handled (decision 0062) |
| `kavach test` and `every`, `after` | The fake clock (spec 11.5) covers `now_ms`, `sleep`, `monotonic_ms`, each round of `every`, and `after` | Fixed |

## Durable agents: the journal

The journal (spec 8.5, `runtime/core/src/journal.rs`) gets one line per event, call outcome and end, written and synced (`sync_data`) before the program goes on.

| Case | What happens today | Status |
|---|---|---|
| Crash or kill between two calls | The event runs again; calls in the journal give their outcome without running | Handled |
| Crash in the middle of writing a line | The cut-off line was skipped, but the next line was written after it on the same line, so it was lost too: a call could run twice, or a finished event run again | Fixed |
| Crash while a call runs, before its outcome is written | The call runs again after the restart: at least once (spec 8.5). Its implementation can send the service `agents.call_key()`, the same in both runs, so the service drops the repeat (spec 14.16) | Handled (decision 0062) |
| An `ask` refused by a limit, in the interpreter | It wasn't journaled (native code journals it), so a run after a crash checked the limit again and could take another path | Fixed |
| The disk is full | A line that can't be written stops the program with a panic naming the journal, as a database stops when its log fails (spec 8.5) | Fixed |
| Power loss right after the journal is made | Its folder is synced when the file is made (on Unix; Windows keeps a new name in its metadata log) | Fixed |
| A handler that always panics | It stays unfinished, so every start runs it again and panics again | Decided ([0073](../decisions/0073-durable-agents-across-restarts.md)): set aside after 3 starts; done (journal `again` lines, `set-aside` file) |
| A long-running agent | When the agent starts, events that ended are dropped and the file is written again (durably), with the next event's number kept | Fixed |
| An `ask` interrupted during its retries | The whole `ask` runs again and its tokens are paid again; a final error (a timeout) is journaled, so a run after a crash gives the same error | Handled |

## Limits: the state file

| Case | What happens today | Status |
|---|---|---|
| Crash or kill | The file is written to `state.json.new`, then renamed over: never half | Handled |
| Power loss | The file wasn't synced before the rename, so it could be empty; an unreadable file counts as no calls, so every limit reset | Fixed (written as `fs.write_durable` does) |
| Kill between a call's effect and the save | A call counts when it starts, and the state file is written then; one that fails stops counting, one a crash cut off stays counted | Fixed (decision 0062) |
| Two programs run the same agent | They share its state file and journal, and overwrite each other's counts | Decided ([0073](../decisions/0073-durable-agents-across-restarts.md)): the folder is locked (`runtime/core/src/agents/lock.rs`); done |

## Files

| Case | What happens today | Status |
|---|---|---|
| `fs.write_text`, `write_bytes` and a crash | Written to a temporary file, then renamed: the old or the new content, never half (spec 14.3) | Handled |
| `fs.write_durable` and power loss | The file and (on Unix) its folder are synced (spec 14.3) | Handled |
| A failed write (a full disk) | The temporary file was left behind when writing or forcing it to the disk failed | Fixed |
| Two tasks writing the same file at once | Their temporary files had the same name; each now has its own (`.name.kavach-<pid>-<n>.tmp`) | Fixed |
| The audit log, `std.log` files | One write per line, not synced, as loggers usually are; a full disk loses lines silently; log rotation is several renames, not one | Decided ([0073](../decisions/0073-durable-agents-across-restarts.md)): the agent's calls are refused until it can be written; done |
| OTLP export | Sent every second; at the end it waits at most 2 s, then what's left is lost | Handled (stated) |

## How a program ends

| Way | Runs the end hooks (OTLP, GPU stop flags, the leak check) | Output flushed |
|---|---|---|
| `main` returns, or `cli.exit` | Yes | Yes |
| A panic in Kavach code | Yes, then exit code 101 | Yes |
| A bug in the runtime itself (a Rust panic) | No: "kavach: the program crashed", code 101 | Yes, before the message |
| Ctrl+C, `SIGTERM`, closing the console | Yes, then exit code 130 or 143; a second Ctrl+C ends it at once ([decision 0063](../decisions/0063-stop-signals-through-libc-and-windows-sys.md)) | Yes |
| Killed (`SIGKILL`, Task Manager), power loss | No | No |
| Out of memory | No: it ends at once with `panic: out of memory (asked for N bytes)` and exit code 101, as spec 5.6 says, through Kavach's own global allocator (`runtime/native/src/rt_memory.rs`); nothing more may be asked of memory, so no end hooks run | Yes |

- **A panic stops only its task**, and a handler's panic only that handler: the send fails with `panicked: …`, the audit log has it, and the agent goes on serving (spec 5.6, decisions [0065](../decisions/0065-a-panic-stops-only-its-task.md) and [0076](../decisions/0076-every-send-to-an-agent-can-fail.md)). A panic in the main task still ends the program. Fixed (`tests/conformance/09-concurrency/a-panic-stops-only-its-task.kav`, `tests/conformance/08-capabilities/a-handler-that-panics-fails-its-send.kav`).
- **Ctrl+C and `SIGTERM` end the program the way `cli.exit` does**: end hooks run, output and logs are flushed. Fixed (tested on Linux, where a test can send the signal).
- **Out of memory is a panic**, as spec 5.6 says. Fixed (tested on Linux, where `ulimit -v` caps a program's memory).

## Memory and other resources

| Resource | When the program ends | Status |
|---|---|---|
| Memory | Left to the operating system, which frees all of it; with `KAVACH_CHECK_LEAKS`, freed one by one and checked | Handled |
| GPU memory | Left to the driver, which frees it | Handled |
| Tasks and threads | Can't outlive their function (spec 9.2); at a crash they stop mid-step | Handled |
| Sockets | Closed by the operating system; a TLS peer sees the connection drop without `close_notify` | Handled |
| A port a server listened on | Can be bound again at once (Rust sets `SO_REUSEADDR` on Unix) | Handled |
| Child processes, lock files | None: the runtime starts none and makes none | Handled |

**Left behind after a crash:** a `.name.kavach-<pid>-<n>.tmp` file from a write cut short (the state file is written the same way now, so no `state.json.new` is left), and `kavach-build-<pid>` (and `kavach-llvm-…`, `kavach-lib-…`) folders in the temp folder from a build stopped with Ctrl+C. None stops a restart. A build removes such folders once they are a day old (`compiler/backend/cranelift/src/link.rs`); the temporary files of writes cut short are left. Fixed for build folders; To do for the files.

## Choices for a maintainer

All seven were decided on 2026-11-04 ([decision 0062](../decisions/0062-crashes-restarts-and-clocks.md)), each as established languages and systems do it (the decision lists the research); the rows marked Maintainer above follow them, and are to do:

1. A wait in a durable handler waits only for what is left after a restart.
2. `now_ms`, `today` and `now` in a durable handler are journaled.
3. Ctrl+C and `SIGTERM` end the program as `cli.exit` does, with exit code 130 or 143.
4. A panic stops only its task (spec 5.6), as in Tokio and Java; a handler's panic is audit-logged.
5. Waits and limit windows use the system's own clock, as Rust, Go, Java and Python do; whether time asleep counts is the system's rule (spec 14.4).
6. A clock set forward while the program was down loosens limits; spec 8.4 says so.
7. A call cut off by a crash runs at least once: an idempotency key for the service, and limits count a call when it starts.
