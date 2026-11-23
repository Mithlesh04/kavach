# runtime/core (`kavach-runtime-core`)

Code that both the interpreter and native code use, so the two give exactly the same results.

| File | Job |
|---|---|
| `src/lib.rs` | How a `Float` prints (spec 1.11) |
| `src/decimal.rs` | `Decimal`: exact base-10 numbers (spec 2.1) |
| `src/wide.rs` | `bits` types wider than 64 bits (`Int<bits: 128>`, spec 2.1): their limits, and each operation checked against them, for the interpreter and native code |
| `src/file_info.rs` | A file's details and permissions, renaming, copying and removing a folder with its contents (spec 14.3), for the interpreter and native code |
| `src/env_file.rs` | Environment variables and `.env` files (spec 10.13, decision 0084): reading a file, and which value wins (`--env`, the real environment, then the file) |
| `src/json.rs` | JSON text (spec 2.12): a strict parser with limits and line/column errors, and the words decoding uses in its errors (its loop in `src/json/parse.rs`, the pieces of text in `src/json/read.rs`, writer in `src/json/write.rs`, tests in `src/json/tests.rs`); values are made by a `Build`er, a tree or a program's own values, or read piece by piece for decoding (`src/json/pull.rs`) |
| `src/input.rs` | A line typed without being shown (`io.read_hidden`, spec 14.6, decision 0036) |
| `src/fs_scope.rs` | What a narrowed `uses` allows while its function runs: folders file access is kept inside (`uses Fs("./data")`, spec 8.1), resolving a path's `..` and links, and the programs that may run (`uses Process("git")`, 14.23); and the spans and seeds of blocks given to a call (4.8): the current span, what a spawned task keeps, ending spans |
| `src/gpu_setting.rs` | What `KAVACH_GPU` and `[run] gpu` ask for (spec 15.6): the system's choice, the CPU, a device's number or a vendor |
| `src/trace.rs` | Span ids and the W3C `traceparent` header (spec 14.12, decision 0107) |
| `src/writers.rs` | Files written piece by piece (`fs.create_writer`, spec 14.3): open writers by number, 64 KB gathered at a time, an unfinished file deleted |
| `src/seeded.rs` | `random.seeded`'s numbers: xoshiro256** seeded through SplitMix64 (spec 14.15) |
| `src/inbox.rs` | Events a thread of the runtime hears (a child's lines, its end), kept for the task waiting for them, which is woken at the next one |
| `src/tasks.rs` | The scheduler tasks take turns by (spec 9.7), for the interpreter and native code alike: passing the turn in `src/tasks/turns.rs`, choosing a worker's next task in `src/tasks/pick.rs` (one turn per worker, decision 0049), tasks waiting for sockets in `src/tasks/io.rs` (decision 0046), tests in `src/tasks/tests.rs` |
| `src/processes.rs` | `spawn process` (spec 9.8, decision 0075): starting the program again to run one call, reading what the child prints, and in the child, which call and its arguments |
| `src/zones.rs` | Time zones (spec 14.4, decision 0064): a zone's offset from UTC at a moment, through `jiff` |
| `src/clipboard.rs` | The system clipboard (spec 14.17, decision 0066): text, through `arboard` |
| `src/serial.rs` | Serial ports (spec 14.18, decision 0068): the open ports by number, through `serial2` |
| `src/text_methods.rs` | Text methods the interpreter and native code share: `capitalize`, `title`, `words`, padding, `truncate`, `is_blank`, `last_index_of`, `count`, removing a prefix or suffix (spec 14.2) |
| `src/regex.rs` | Regular expressions (spec 14.8): a linear-time engine, and an advanced one with a backtracking limit |
| `src/agents.rs` | The bookkeeping of agents' guards (spec 8.3, 8.5, 8.6): limit totals over rolling windows, asking a person; the audit log in `src/agents/audit.rs`, the state file in `src/agents/state.rs` (tests in `src/agents/tests.rs`) |
| `src/app.rs` | `std.app`'s folders by each system's rule, and the crash report a panic writes (spec 14.30) |
| `src/chrome_trace.rs` | Spans written as a Chrome trace when `KAVACH_TRACE` names a file (spec 14.12) |
| `src/fine_clock.rs` | A nanosecond clock that never jumps, and the spin at the end of a precise wait (spec 14.4) |
| `src/journal.rs` | The journal of a durable agent (spec 8.5, decision 0025): events, call outcomes and ends as JSON lines, and the events a crash left unfinished |
| `src/log.rs` | Where `std.log` writes (spec 14.12): the console, and a file set aside when it grows past a size (tests in the file); whether standard output shows colors (`io.colors()`, spec 14.6) |
| `src/math.rs` | `std.math`'s inexact functions (`pow`, `sin` to `atan2`, `exp`, `ln`) from the pure-Rust `libm` crate, the same bits on every system (spec 14.5, decision 0122) |
| `src/profiler.rs` | The hooks a live profiler sets (spec 14.12, decision 0123): a span begins and ends, and the GPU's timed work; `runtime/tracy` sets them |
| `src/base64.rs` | `std.base64` (spec 14.20): standard and URL-safe base64, decoded strictly |
| `src/compress.rs` | `std.compress` (spec 14.21): gzip, deflate, zstd, brotli and lz4, decoding stopped at a limit |
