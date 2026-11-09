# runtime/core (`kavach-runtime-core`)

Code that both the interpreter and native code use, so the two give exactly the same results.

| File | Job |
|---|---|
| `src/lib.rs` | How a `Float` prints (spec 1.11) |
| `src/decimal.rs` | `Decimal`: exact base-10 numbers (spec 2.1) |
| `src/json.rs` | JSON text (spec 2.12): a strict parser with limits and line/column errors, and the words decoding uses in its errors (its loop in `src/json/parse.rs`, the pieces of text in `src/json/read.rs`, writer in `src/json/write.rs`, tests in `src/json/tests.rs`); values are made by a `Build`er, a tree or a program's own values, or read piece by piece for decoding (`src/json/pull.rs`) |
| `src/input.rs` | A line typed without being shown (`io.read_hidden`, spec 14.6, decision 0036) |
| `src/fs_scope.rs` | Folders file access is kept inside (`uses Fs("./data")`, spec 8.1): resolving a path's `..` and links, and checking it |
| `src/tasks.rs` | The scheduler tasks take turns by (spec 9.7), for the interpreter and native code alike: passing the turn in `src/tasks/turns.rs`, choosing a worker's next task in `src/tasks/pick.rs` (one turn per worker, decision 0049), tasks waiting for sockets in `src/tasks/io.rs` (decision 0046), tests in `src/tasks/tests.rs` |
| `src/processes.rs` | `spawn process` (spec 9.8, decision 0075): starting the program again to run one call, reading what the child prints, and in the child, which call and its arguments |
| `src/zones.rs` | Time zones (spec 14.4, decision 0064): a zone's offset from UTC at a moment, through `jiff` |
| `src/clipboard.rs` | The system clipboard (spec 14.17, decision 0066): text, through `arboard` |
| `src/serial.rs` | Serial ports (spec 14.18, decision 0068): the open ports by number, through `serial2` |
| `src/regex.rs` | Regular expressions (spec 14.8): a linear-time engine, and an advanced one with a backtracking limit |
| `src/agents.rs` | The bookkeeping of agents' guards (spec 8.3, 8.5, 8.6): limit totals over rolling windows, asking a person; the audit log in `src/agents/audit.rs`, the state file in `src/agents/state.rs` (tests in `src/agents/tests.rs`) |
| `src/journal.rs` | The journal of a durable agent (spec 8.5, decision 0025): events, call outcomes and ends as JSON lines, and the events a crash left unfinished |
| `src/log.rs` | Where `std.log` writes (spec 14.12): the console, and a file set aside when it grows past a size (tests in the file); whether standard output shows colors (`io.colors()`, spec 14.6) |
