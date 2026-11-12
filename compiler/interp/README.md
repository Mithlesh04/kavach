# compiler/interp (`kavach-interp`)

A development interpreter ([decision 0014](../../docs/decisions/0014-development-interpreter.md)). It runs a checked program straight from its syntax tree for `kavach run`, `kavach test` (including generated tests) and the conformance tests. Shipped programs will be compiled to native code; the interpreter is the reference they must match.

It follows the spec's run-time rules: integer overflow and division by zero panic (5.6), contracts are checked (7.4), exact `Decimal` math (2.1), values print like literals (1.11), and exit codes are 1 for an error from `main` and 101 for a panic (5.8).

Not supported yet: agents, capabilities, `ask` and `unsafe` (Phase 2 and the native backend).

| File | Job |
|---|---|
| `src/lib.rs` | `run` and `test`, run on a thread with a large stack |
| `src/interp.rs` | State, function calls with `require`/`ensure`, invariants, test mode |
| `src/eval.rs` | Expressions |
| `src/stmts.rs` | Statements, assignment into fields and lists, pattern matching |
| `src/drops.rs` | `Drop` (spec 6.1): the identity copies share, and running `drop` at statement boundaries |
| `src/calls.rs` | Calls: functions, closures, records, variants, methods |
| `src/ops.rs` | Operators, with overflow checks per integer type |
| `src/builtins.rs` | Prelude functions and methods of built-in types |
| `src/bytes.rs` | `Bytes` (spec 2.13) |
| `src/net.rs` | `std.net` (spec 14.10): TCP, with the code in `runtime/core` |
| `src/fakes.rs` | The fakes of `Fs` and `Clock` in generated tests (spec 11.5): files in memory, a clock that jumps |
| `src/generics.rs` | The type arguments of a generic function's call, for what needs the type at run time (`json.decode<T>`) |
| `src/fake_net.rs` | The fake network of generated tests (spec 11.5): connections that fail, or reach a fake server answering HTTP with HTTP |
| `src/files.rs` | Files read as a `for` loop goes: `fs.lines`, `fs.chunks` (spec 14.3), from disk or from a generated test's memory |
| `src/tasks.rs` | Tasks (spec 9): each on its own thread, taking turns through the scheduler in `runtime/core` |
| `src/task_errors.rs` | An error nobody reads (spec 9.1): printed when the last holder of its task lets it go |
| `src/timeouts.rs` | `x.timeout(d)` (spec 9.6): a wait or a network call for at most `d`; `spawn … within d` |
| `src/processes.rs` | `spawn process f(args)` (spec 9.8): the call run in a child started with the command `kavach run` gives, its lines printed in turn; in the child, running only that call |
| `src/channels.rs` | Channels (spec 9.4): `put` waits while full, `take` while empty |
| `src/value.rs` | Run-time values and how they print |
| `src/map.rs` | Maps (and sets): insertion order plus a hash index |
| `src/stack.rs` | How deep calls may go: the stack in use, measured, before a call is a stack overflow (spec 3.8) |
| `src/std_lib.rs` | The standard library: more methods, sets, `std` modules (spec 14) |
| `src/std_agents.rs` | Approval requests answered in code: `agents.waiting` and `answer` (spec 14.16) |
| `src/std_devices.rs` | The clipboard and serial ports (spec 14.17, 14.18) |
| `src/numbers.rs` | Methods of numbers: `abs`, `near`, `min`, `max`, `clamp`, `div_floor`, `mod`, `round`, `round_even` (spec 14.2) |
| `src/std_time.rs` | `std.time` (spec 14.4): clocks, sleeping, intervals, `time.after` |
| `src/json_decode.rs` | `json.decode<T>`: JSON into typed values, with defaults and exact error paths |
| `src/json.rs` | `Json` and `std.json` (spec 2.12): parsing, writing, encoding, literals, `j["key"]`, printing |
| `src/cli.rs` | `cli.parse<T>()`: options from a record's fields, and the usage text |
| `src/agents.rs` | Agents (spec 8.2, 8.3): `start`, handlers, capability calls through the guard, dry runs and test fakes |
| `src/guard.rs` | An agent's limits and approvals, evaluated per call; the bookkeeping is `runtime/core`'s |
| `src/ai.rs` | `ask<T>` (spec 8.4): through the guard, the model call, the answer checked against `T` |
| `src/journal.rs` | Durable agents (spec 8.5, decision 0025): events and calls written to the journal (`runtime/core`), and events run again after a crash |
| `src/run.rs` | The two ways to run a file: `main` and its tests (`example` and `test`) |
| `src/std_crypto.rs` | `std.crypto` (spec 14.19), from `runtime/net` |
| `src/std_base64.rs` | `std.base64` (spec 14.20), from `runtime/core` |
| [src/testing/](src/testing/README.md) | Generated tests: values, shrinking, found cases, fix suggestions (spec 11) |

Tests live in [tools/cli/tests/conformance.rs](../../tools/cli/tests/conformance.rs): every conformance test, plus examples 01–04.
