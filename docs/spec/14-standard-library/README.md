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

## Open questions

- Matching on kinds of the other standard errors (`RegexError`, `CliError`).
- `std.time`: formats other than ISO 8601 (`4 Nov 2026`), for both reading and writing.
- `std.clipboard`: images, and other kinds of content (HTML, files).
- `std.devices`: a serial wait that lets other tasks run (as the network's do, 9.7), and line settings other than 8N1.
- Streamed reads that fail part way: an error the loop can see, rather than a panic?
- `std.term` (14.13): turning colors on with a setting even when output is not a terminal.
- `std.term`: background colors and colors by number (256 colors, RGB).
- `std.term`: colors in the old Windows console (conhost without virtual terminal mode).
