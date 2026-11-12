# stdlib/

The standard library, imported as `std.<module>`, e.g. `use std.fs`. Its contents are in [spec chapter 14](../docs/spec/14-standard-library/README.md).

**Status:** the modules written in Kavach are `json/json.kav`, `text/text.kav`, `collections/collections.kav`, `math/math.kav` (with the runtime's `sqrt`, `pow`, rounding, `pi` and `e`) `fs/fs.kav` (the `FsError` union; the file operations need the operating system, so the runtime provides them), `time/time.kav` (`Duration`, and dates and times of day), `test/test.kav` (`Sample` and `Gen`) and `term/term.kav` (colors, on only when the runtime's `io.colors()` says so; progress bars and prompts). The compiler adds the ones a program uses (`compiler/driver`); `use std.math` reaches both halves, so a program can't tell which part is which. Each file has `example`s and contracts, and `kavach test` runs them (`tools/cli/tests/stdlib.rs`). The modules that need the operating system (`fs`, `time`, `io`, `cli`) are provided by the runtime.

| Folder | Module | Contents | Phase |
|---|---|---|---|
| `core/` | prelude | `Int`, `String`, `Option`, errors, `print`, `debug`, `ask`, … (always available) | 1 |
| `collections/` | `std.collections` | `collections.kav`: chunks, zip, unique items, counting, flattening | 1 |
| `text/` | `std.text` | `text.kav`: padding, words, capitalizing, truncating, blank text | 1 |
| `math/` | `std.math` | `math.kav`: `gcd`, `lcm`, `factorial`, `is_prime`, `mean` | 1 |
| `time/` | `std.time`, `std.time.digits` | `digits.kav`: the digits and offsets of times as text; `time.kav`: `Duration` (`5.seconds`), `Date`, `Time`, `DateTime`, `Weekday`; clocks, sleeping and intervals come from the runtime (spec 14.4) | 1 |
| `fs/` | `std.fs` | `fs.kav`: the `FsError` union file operations fail with (spec 14.3) | 1 |
| `io/` | `std.io` | Files, console, environment | 1 |
| `test/` | `std.test` | `test.kav`: `Sample`, for types that make their own generated values, and `Gen`, the seeded choices they make them from (spec 14.7); the numbers come from the runtime | 2 |
| `json/` | `std.json` | `json.kav`: the `Json` and `JsonError` types and reading a `Json`, written in Kavach; every program has them. Parsing and writing are in the compiler and runtime (spec 2.12) | 1 |
| `net/` | `std.net` | `net.kav`: the `NetError` union; TCP, UDP and QUIC come from the runtime (spec 14.10). HTTP is in `http/` | 2 |
| `http/` | `std.http`, `std.http.server`, `std.http.multipart`, `std.http.upgrade`, `std.http.message`, `std.http.hpack`, `std.http.h2`, `std.http.h2.wire`, `std.http.h3` | `http.kav`: HTTP/1.1 and HTTP/2 clients and servers, and HTTP/3 clients, on `std.net`, written in Kavach, with typed JSON calls and sessions that keep a connection open (spec 14.11); `server.kav`: the server `http.serve` runs, each client held to its `ServeLimits` ([decision 0087](../docs/decisions/0087-http-headers-and-server-limits.md)); `multipart.kav`: forms read part by part (`http.form`, [decision 0090](../docs/decisions/0090-forms-read-part-by-part.md)); `upgrade.kav`: WebSockets and server-sent events ([decision 0092](../docs/decisions/0092-websockets-and-server-sent-events.md)); `message.kav`: reading HTTP/1.1 messages, chunked bodies too, and where each ends; `h2.kav`: HTTP/2 requests and responses, whole or piece by piece; `h2wire.kav`: its frames, streams and flow control; `hpack.kav`: HTTP/2 header compression; `h3.kav`: HTTP/3 requests over QUIC, with QPACK ([decision 0022](../docs/decisions/0022-http2-and-http3.md)) | 2 |
| `term/` | `std.term` | `term.kav`: colors and styles for the terminal, off when output is not a terminal; progress bars; prompts (spec 14.13) | 1 |
| [`log/`](log/README.md) | `std.log` | `log.kav`: log events are records the program defines; levels and the console sink, written in Kavach (spec 14.12) | 2 |
| `ai/` | `std.ai` | `ai.kav`: `AiError`, what `ask<T>` fails with, and `Model`, the model and settings (effort, tokens, thinking, instructions, stop text, timeout, retries, address and key, extra request fields) an `ask` uses; the model call is in the runtime (spec 8.4) | 2 |
| `devices/` | `std.devices` | `devices.kav`: `SerialPort`, its methods, and `DeviceError`; ports are opened, read and written by the runtime, behind `Serial` (spec 14.18) | 2 |
| `agents/` | `std.agents` | `agents.kav`: `ApprovalRequest`, `ApprovalError`, and `waiting` and `answer`, which answer agents' approval requests in code, behind `Approvals`; the requests are found and answered by the runtime, as is `call_key` (spec 14.16) | 2 |
| `clipboard/` | `std.clipboard` | `clipboard.kav`: the `ClipboardError` union; reading and writing the clipboard come from the runtime, behind `Clipboard` (spec 14.17) | 2 |
| [`random/`](random/README.md) | `std.random` | `random.kav`: whole numbers in a range, floats, picks, shuffles and bytes, from the operating system's secure generator, seeded under `kavach test` (spec 14.15) | 2 |
| `crypto/` | `std.crypto` | `crypto.kav`: the `CryptoError` union; hashes, HMAC, passwords (Argon2id), signatures and AES-GCM come from `ring` and `argon2` in the runtime (spec 14.19, [decision 0093](../docs/decisions/0093-std-crypto.md)) | 2 |
| `base64/` | `std.base64` | `base64.kav`: the `Base64Error` union; encoding and decoding are built into the runtime (`runtime/core/src/base64.rs`, spec 14.20) | 2 |
| `tasks/` | `std.tasks` | `tasks.kav`: `Taken<T>`, what a channel's `take()` gives (spec 9.4, [decision 0097](../docs/decisions/0097-take-gives-taken.md)); every program that uses a channel gets it, and tasks and channels themselves are built in | 2 |
| [`grpc/`](grpc/README.md) | `std.grpc`, `std.grpc.frames`, `std.grpc.protobuf` | `grpc.kav`: gRPC clients and servers from Kavach types; `frames.kav`: message prefixes and status trailers; `protobuf.kav`: Protocol Buffers from a type's JSON Schema, written in Kavach on `std.http.h2` (spec 14.14, [decision 0027](../docs/decisions/0027-grpc-as-a-module.md)) | 3 |

Rule: the standard library stays **small**. Anything that can live as a package lives as a package.
