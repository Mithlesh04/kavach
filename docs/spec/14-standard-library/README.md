# 14. Standard Library

The standard library stays **small**: anything that can be a package is a package. This chapter lists what Phase 1 provides, and `std.net` (14.10) from Phase 2. Later phases add `std.log`, HTTP, TLS and more ([platform plan](../../vision/platform.md)).

Each section is a file of its own: new sections go at the end, with the next number.

| Section | File |
|---|---|
| 14.1 How it is organized | [01-organization.md](01-organization.md) |
| 14.2 Methods of built-in types | [02-built-in-methods.md](02-built-in-methods.md) |
| 14.3 std.fs | [03-fs.md](03-fs.md) |
| 14.4 std.time | [04-time.md](04-time.md) |
| 14.5 std.math | [05-math.md](05-math.md) |
| 14.6 std.io and std.cli | [06-io-and-cli.md](06-io-and-cli.md) |
| 14.7 std.test | [07-test.md](07-test.md) |
| 14.8 std.regex | [08-regex.md](08-regex.md) |
| 14.9 std.text and std.collections | [09-text-and-collections.md](09-text-and-collections.md) |
| 14.10 std.net | [10-net.md](10-net.md) |
| 14.11 std.http | [11-http.md](11-http.md) |
| 14.12 std.log | [12-log.md](12-log.md) |
| 14.13 std.term | [13-term.md](13-term.md) |
| 14.14 std.grpc | [14-grpc.md](14-grpc.md) |
| 14.15 std.random | [15-random.md](15-random.md) |
| 14.16 std.agents | [16-agents.md](16-agents.md) |
| 14.17 std.clipboard | [17-clipboard.md](17-clipboard.md) |
| 14.18 std.devices | [18-devices.md](18-devices.md) |
| 14.19 std.crypto | [19-crypto.md](19-crypto.md) |
| 14.20 std.base64 | [20-base64.md](20-base64.md) |
| 14.21 std.compress | [21-compress.md](21-compress.md) |
| 14.22 std.window | [22-window.md](22-window.md) |
| 14.23 std.process | [23-process.md](23-process.md) |
| 14.24 std.gpu | [24-gpu.md](24-gpu.md) |
| 14.25 std.system | [25-system.md](25-system.md) |
| 14.26 std.input | [26-input.md](26-input.md) |
| 14.27 std.audio | [27-audio.md](27-audio.md) |
| 14.28 std.vec | [28-vec.md](28-vec.md) |
| 14.29 std.image | [29-image.md](29-image.md) |
| 14.30 std.app | [30-app.md](30-app.md) |
| 14.31 std.font | [31-font.md](31-font.md) |
| 14.32 std.reflect | [32-reflect.md](32-reflect.md) |
| 14.33 std.tensor: Kavach's one array type, as NumPy's | [33-tensor.md](33-tensor.md) |
| 14.34 std.table | [34-table.md](34-table.md) (on the GPU: not built yet) |

## Open questions

- Matching on kinds of the other standard errors (`RegexError`, `CliError`).
- `std.time`: formats other than ISO 8601 (`4 Nov 2026`), for both reading and writing.
- `std.clipboard`: images, and other kinds of content (HTML, files).
- `std.devices`: a serial wait that lets other tasks run (as the network's do, 9.7), and line settings other than 8N1.
- `std.devices` (14.18, chosen 2026-11-22, for a maintainer's review): baud 0 is refused, not taken as "hang up"; a file or folder is `Failed(path, "not a serial port")` on every system; `read(0)` and a negative `max` give empty `Bytes` at once, as negative counts do in 14.2.
- Streamed reads that fail part way: an error the loop can see, rather than a panic?
- `std.term` (14.13): turning colors on with a setting even when output is not a terminal.
- `std.term`: background colors and colors by number (256 colors, RGB).
- `std.term`: colors in the old Windows console (conhost without virtual terminal mode).
- Grouping and statistics (14.2): percentiles other than the quartiles. `xs.percentile(f, p)` (0 to 100), as to-do 28 plans for tensors, or `quantile(f, q)` (0 to 1), as pandas names it; one name for lists, tables and tensors.
- 14.2: `xs.sum()`, `xs.mean()` and the others with no lambda, for a list of numbers, in place of `xs.mean(x => x)`.
- 14.2: methods on dicts beyond `map_values` and `to_list` (`filter` by key or value, `sort_by` of the entries).
- 14.2: `mean` or `median` in generic code, where the lambda gives a type parameter `N`. The result's type depends on `N` (a `Float` for whole numbers, `N` itself for floats and `Decimal`), and a generic function can't name it. Today the call's context or its type arguments say (`xs.mean<N, Float>(f)`), and nothing says it otherwise. A compile error asking for the type, always a `Float`, or a bound on `N`?
- 14.2's sortable types (chosen 2026-11-22, for a maintainer's review): exactly the types 1.8 gives an order, so `sort`, `sort_by`, `min` and `max` take lists of lists, rings, `Bytes` and text views too, not only numbers, text, characters, `Bool` and tuples.
- 14.2: `var` and `std` with a negative `correction:`, which divides by more than the count. Allowed, as NumPy allows a negative `ddof`, or a panic?
