# runtime/native (`kavach-runtime-native`)

The run-time library of native Kavach programs ([decision 0016](../../docs/decisions/0016-native-runtime-and-executables.md)): what compiled code calls for heap values, printing, panics and errors. Its output must match the interpreter exactly.

It is built two ways: as a Rust library the compiler uses for `kavach run --native`, and as a static library that `kavach build` links into every executable. Every function native code calls is an unmangled `extern "C"` symbol, so both ways find it.

This is the only crate with `unsafe` Rust: exporting unmangled symbols, calling the compiled entry function, and turning object pointers back into objects. Each use has a `SAFETY:` comment.

| File | Job |
|---|---|
| `src/lib.rs` | `start`, `run_entry`, and `kv_main`, the start of an executable; `kv_lib_enter` and `kv_lib_leave`, around each call into a library (decision 0026) |
| `src/heap.rs` | Object layout (read by generated code), reference counts, copy-on-write, freeing, the leak counter |
| `src/shapes.rs` | The program's heap types; reading text, lengths, items and slots out of objects |
| `src/text_items.rs` | Items of a list of text: short text inside the item, longer text as a pointer ([design](../../docs/architecture/flat-lists.md#lists-of-text)) |
| `src/radix.rs` | Radix sort for lists of integers |
| `src/lists.rs` | A list's buffer: changing it as a `Vec`, adding to it |
| `src/flat.rs` | Flat lists of plain records: boxing items for generic code, unboxing them back ([design](../../docs/architecture/flat-lists.md)) |
| `src/pool.rs` | A pool for small objects: free lists per size, cut from 4 MB blocks on huge pages |
| `src/small_blocks.rs` | Small blocks for all that Rust code allocates: a free list per size per thread, cut from regions on huge pages |
| `src/text.rs` | Text contents: up to 30 bytes inside the object, longer text in a heap buffer |
| `src/table.rs` | The hash table behind maps and sets ([design](../../docs/architecture/hash-maps.md)); `src/table/grow.rs` doubles an index of integer keys in place |
| `src/rt.rs` | Printing, panics, integer checks, the stack limit |
| `src/rt_data.rs` | Run-time functions for heap values: text, lists, records, optionals |
| `src/rt_numbers.rs` | `Decimal`, `wrapping_*`/`checked_*`, `parse` |
| `src/rt_errors.rs` | `context` and the error of a failed `parse` |
| `src/rt_text.rs` | More methods of text and lists: `replace`, `sort`, `join`, … (spec 14.2) |
| `src/rt_std.rs` | `std.fs` (atomic writes), `std.time`, `std.math`, `std.io`, `std.cli` |
| `src/rt_memory.rs` | The global allocator: an allocation the system refuses is a panic, `panic: out of memory` with exit code 101 (spec 5.6), not Rust's abort |
| `src/rt_stop.rs` | Ctrl+C, `SIGTERM` and closing the console (spec 5.8, decision 0063): the end hooks run, then exit code 130 or 143; a second one ends at once |
| `src/rt_zone.rs` | Time zones (spec 14.4): `time.zone_offset` through `runtime/core`; its own entry, so only programs that use zones link `jiff` and the zone data |
| `src/rt_clipboard.rs` | The clipboard (spec 14.17): `read_text` and `write_text` through `runtime/core`; its own entry, so only programs that use it link `arboard` |
| `src/rt_process.rs` | `spawn process f(args)` (spec 9.8): the call run in a child, its value or error given back as JSON; its own entry, so only programs that use it link it |
| `src/rt_serial.rs` | Serial ports (spec 14.18): by number, through `runtime/core`; its own entry, so only programs that use them link `serial2` |
| `src/rt_tasks.rs` | Tasks (spec 9), taking turns through the scheduler in `runtime/core` |
| `src/unwind_tables.rs` | Registers the unwind tables of code the JIT made (decision 0065), so a panic stops only its task in `kavach run --native` too |
| `src/rt_task_errors.rs` | An error nobody reads (spec 9.1): printed when its task is freed, or when the program ends |
| `src/rt_waits.rs` | Waiting: for a time, a socket or a channel, while other tasks run; refused inside `Shared.update` |
| `src/rt_coroutines.rs` | Tasks as coroutines on the main thread, with `corosensei` (decision 0048): the context each switch saves and restores |
| `src/rt_drops.rs` | `Drop` (spec 6.1): a value's last release waits for its `drop`, which runs at the next statement boundary |
| `src/rt_agents.rs` | Agents (spec 8.3): started agents' guards (bookkeeping from `runtime/core`), the stack of running agents, each call's checks |
| `src/rt_approvals.rs` | Approval requests answered in code (spec 14.16): `agents.waiting` and `answer`, in the project's agents folder, through `runtime/core` |
| `src/rt_replays.rs` | Events run again after a restart (decision 0072): each a task `start` waits for until it ends or asks a person; one left waiting is joined by a send of the same event, and its outcome goes to the audit log |
| `src/rt_ai.rs` | `ask<T>` (spec 8.4): the model call from `runtime/core`, the answer decoded as `T`, asked again when it doesn't fit |
| `src/opaque.rs` | A feature's own values (sockets, regexes) in heap objects, freed through the feature's functions so other programs don't link its code (decision 0024) |
| `src/rt_channels.rs` | Channels (spec 9.4), as in the interpreter |
| `src/rt_bytes.rs` | `Bytes` (spec 2.13): raw data in one buffer |
| `src/rt_net.rs` | `std.net` (spec 14.10): TCP, with the code in `runtime/core` |
| `src/rt_files.rs` | Files read as a `for` loop goes: `fs.lines`, `fs.chunks` (spec 14.3) |
| `src/send.rs` | Deep copies of a value for a task that runs ahead on another thread; cells that copies share (`Shared`, channels, tasks) |
| `src/live.rs` | Counting live objects for the leak check, per thread when tasks run at once |
| `src/rt_regex.rs` | `std.regex`, with the engine from `runtime/core` |
| `src/rt_json_decode.rs` | `json.decode<T>`: JSON into typed values by their shape, with record defaults passed in as templates; it tries `src/rt_json_pull.rs` first, which reads straight from the text |
| `src/rt_json_build.rs` | `json.parse` straight into `Json` values: maps made at their final size, repeated values shared |
| `src/rt_json.rs` | `std.json`: JSON text to and from `Json` values, with the parser from `runtime/core` |
| `src/rt_log.rs` | `std.log`'s lines (spec 14.12): written as `[log]` in `kavach.toml` says, with `runtime/core`'s log writer |
| `src/rt_journal.rs` | Durable agents (spec 8.5): each agent's journal (`runtime/core`), the events left unfinished, values to and from JSON; its own entry, so only programs with a durable agent link it |
| `src/rt_ptr.rs` | Raw memory (spec 6.4): `alloc` and `free` are C's `calloc` and `free`; text copied for C with a zero byte after it, and C's text copied back (10.7); text and errors across a library's boundary, `<package>_free` and `<package>_last_error` (10.10) |
| `src/rt_otlp.rs` | Log events to an OpenTelemetry collector (spec 14.12), with `runtime/net`'s exporter; its own entry, so only programs that export link it |
| `src/rt_cli.rs` | `cli.parse<T>()`: reading and converting the arguments |
| `src/rt_maps.rs` | Map and set operations, `Shared`, `list[i] = v` |
| `src/rt_maps/known_keys.rs` | `map["key"]` for a constant key: each key's hash and entry kept by its address, and the key object held once found twice, so a JSON object's key is found by comparing addresses |
| `src/display.rs` | Printing (spec 1.11), equality and ordering of heap values |
