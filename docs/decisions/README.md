# Decision Records

One file per important decision: what was decided, why, and what was rejected. Future contributors, and our future selves, can see *why* Kavach is the way it is.

## Index

| # | Decision | Status |
|---|---|---|
| [0001](0001-braces-for-blocks.md) | Braces for blocks, no semicolons | Accepted |
| [0002](0002-static-types-with-inference.md) | Static types with inference | Accepted |
| [0003](0003-memory-model.md) | Automatic memory without GC, with opt-in control | Accepted |
| [0004](0004-compiler-written-in-rust.md) | Write the first compiler in Rust | Accepted |
| [0005](0005-backends-cranelift-llvm-wasm.md) | Cranelift for development, LLVM for release and WASM | Accepted |
| [0006](0006-name-and-file-extension.md) | Name: Kavach; extension: `.kav` | Accepted |
| [0007](0007-errors-as-values.md) | Errors are values; no exceptions | Accepted |
| [0008](0008-capabilities-in-signatures.md) | Capabilities are part of function signatures | Accepted |
| [0009](0009-size-limits-and-enforced-architecture.md) | 500-line file limit and enforced architecture | Accepted |
| [0010](0010-tests-from-contracts.md) | Tests are generated from types and contracts | Accepted |
| [0011](0011-license-apache-or-mit.md) | License: Apache-2.0 OR MIT | Accepted |
| [0012](0012-return-types-are-written.md) | Return types are always written | Accepted |
| [0013](0013-core-language-decisions.md) | Core language decisions for Phase 1 | Accepted (labeled `break`: replaced by 0054) |
| [0014](0014-development-interpreter.md) | A development interpreter before the native backend | Accepted |
| [0015](0015-token-efficient-ai-tooling.md) | Token efficiency for AI is a design goal | Accepted |
| [0016](0016-native-runtime-and-executables.md) | A native runtime library, linked into executables | Accepted |
| [0017](0017-native-platform-and-speed-targets.md) | A native platform, and speed targets we can prove | Accepted |
| [0018](0018-llvm-backend-through-llvm-tools.md) | The LLVM backend writes LLVM IR text and uses rustup's LLVM tools | Accepted |
| [0019](0019-json-values.md) | A built-in `Json` union for dynamic values | Accepted |
| [0020](0020-tasks-take-turns.md) | Tasks take turns, in a defined order, before they use all cores | Accepted |
| [0021](0021-tls-through-rustls.md) | TLS through rustls, with root certificates built in | Accepted |
| [0022](0022-http2-and-http3.md) | HTTP/2 written in Kavach; HTTP/3 over quinn-proto | Accepted |
| [0023](0023-logging-needs-no-capability.md) | Logging needs no capability | Accepted |
| [0024](0024-pay-only-for-what-you-use.md) | Pay only for what you use: modules, functions and runtime features | Accepted |
| [0025](0025-agents-resume-by-replaying-a-journal.md) | Agents resume after a crash by replaying a journal | Accepted |
| [0026](0026-a-library-for-other-languages.md) | A library for other languages through the C ABI | Accepted |
| [0027](0027-grpc-as-a-module.md) | gRPC as a Kavach module, driven by a type's JSON Schema | Accepted |
| [0028](0028-anonymous-records.md) | Anonymous records: Python's speed of writing, fully typed | Accepted |
| [0029](0029-interface-values.md) | Interface values: runtime polymorphism without inheritance | Accepted |
| [0030](0030-byte-packed-lists.md) | Byte-packed lists: one byte per byte | Accepted |
| [0031](0031-loops-on-all-cores.md) | Math loops on all cores, with the plain loop's exact results | Accepted |
| [0032](0032-json-on-a-tape.md) | JSON on a tape: one buffer per document | Accepted |
| [0033](0033-docs-site-in-the-repo.md) | The docs site lives in this repository | Accepted; point 1 replaced by [0047](0047-docs-one-idea-per-page.md) |
| [0034](0034-const-and-let.md) | `const` for values that never change, `let` for variables (no `var`) | Accepted |
| [0035](0035-library-calls-overlap.md) | Library calls compute at the same time, and take turns for effects | Accepted |
| [0036](0036-hidden-input-through-rpassword.md) | Passwords are read unseen through `rpassword` | Accepted |
| [0037](0037-destructuring-as-javascript-does.md) | Destructuring as JavaScript does, with Kavach's types | Accepted |
| [0038](0038-inout-parameters.md) | `inout` parameters: a function changes its caller's variable, safely | Accepted |
| [0039](0039-equality-is-deep-and-fast.md) | Equality compares contents, ignores map order, and skips shared parts | Accepted |
| [0040](0040-chained-comparisons.md) | Comparisons chain, as in Python | Accepted |
| [0041](0041-bigint-is-built-in.md) | `BigInt` is a built-in number type, and big constants infer it | Accepted |
| [0042](0042-decimal-has-no-size-limit.md) | `Decimal` has no size limit | Accepted |
| [0043](0043-constants-are-exact.md) | Constants are worked out exactly, then rounded once | Accepted |
| [0044](0044-gpu-through-wgpu.md) | GPU code: `gpu fn`, run by `map`, through `wgpu` | Accepted |
| [0045](0045-packed-lists-of-small-numbers.md) | Packed lists of 16- and 32-bit numbers | Accepted |
| [0046](0046-io-event-loop-through-mio.md) | The I/O event loop, through `mio` | Accepted |
| [0047](0047-docs-one-idea-per-page.md) | The docs teach one idea per page: parts → pages → sub-pages | Accepted |
| [0048](0048-tasks-as-coroutines-per-core.md) | Tasks as coroutines, then coroutines on every core | Accepted |
| [0049](0049-tasks-on-all-cores-keep-the-programs-order.md) | Tasks on all cores keep the order the program gives | Accepted |
| [0050](0050-match-on-an-error-names-its-type.md) | A `match` on an `Error` names the error's type | Accepted |
| [0051](0051-number-and-text-defaults-keep-their-speed.md) | Number and text defaults keep their speed | Accepted |
| [0052](0052-shared-update-holds-its-lock-and-never-waits.md) | `Shared.update` holds its lock, and never waits | Accepted |
| [0053](0053-terminal-questions-are-prompt.md) | Terminal questions are `prompt`, not `ask` | Accepted |
| [0054](0054-loops-can-have-labels.md) | Loops can have labels | Accepted |
| [0055](0055-gpu-results-stay-until-read.md) | GPU results stay on the GPU until the CPU reads them | Accepted |
| [0056](0056-the-model-is-chosen-in-code.md) | The AI model is chosen in the code | Accepted |
| [0057](0057-ask-puts-the-type-in-the-prompt.md) | `ask` puts the type in the prompt, and checks the answer | Accepted |
| [0058](0058-dates-in-std-time.md) | Dates in `std.time`, and reading the clock needs no capability | Accepted |
| [0059](0059-model-settings-follow-the-providers.md) | A `Model`'s settings follow the providers | Accepted |
| [0060](0060-agent-settings-are-chosen-in-code.md) | An agent's settings are chosen in code | Accepted |
| [0061](0061-gpu-maps-dont-wait.md) | A GPU `map` whose list stays on the GPU doesn't wait for it | Accepted |
| [0062](0062-crashes-restarts-and-clocks.md) | What crashes, restarts and clock changes do | Accepted |
| [0063](0063-stop-signals-through-libc-and-windows-sys.md) | Ctrl+C and `SIGTERM` through `libc` and `windows-sys` | Accepted |
| [0064](0064-time-zones-through-jiff.md) | Time zones through `jiff`, with the operating system's rules | Accepted |
| [0065](0065-a-panic-stops-only-its-task.md) | A panic stops only its task: how native code gets there | Accepted |
| [0066](0066-clipboard-through-arboard.md) | The clipboard through `arboard` | Accepted |
| [0067](0067-streaming-data-through-the-gpu.md) | Streaming data through the GPU | Accepted |
| [0068](0068-serial-ports-through-serial2.md) | Serial ports through `serial2` | Accepted |
| [0069](0069-each-task-chooses-how-it-runs.md) | Each task chooses how it runs, where it is spawned (`spawn`, `spawn thread`, `spawn cores`) | Accepted |
| [0070](0070-variants-are-written-with-their-type.md) | Variants are written with their type: `Status.Pending`, in `match` cases too | Accepted |
| [0071](0071-json-numbers-keep-their-digits.md) | JSON numbers keep their digits (`Json.Number(digits)`); `json.parse(text, comments: true)` for JSONC | Accepted |
| [0072](0072-approvals-that-wait-across-restarts.md) | Approvals that wait across restarts: a journaled wait with a deadline, answered by command or in code | Accepted |
| [0073](0073-durable-agents-across-restarts.md) | Durable agents: an event that never finishes is set aside, one program per agent, an audit log that must be written | Accepted |
| [0074](0074-gimli-for-unwind-tables.md) | `gimli` writes the unwind tables of `kavach build` (decision 0065, step 1) | Accepted |
| [0075](0075-cancelling-tasks.md) | Cancelling tasks: `task.cancel()` for every kind, checks so stuck loops stop, `spawn process` for code Kavach can't check, errors nobody reads are printed | Accepted |
| [0076](0076-every-send-to-an-agent-can-fail.md) | Every send to an agent can fail: a handler's panic is audit-logged and the send fails with `panicked: …` | Accepted |
| [0077](0077-kernels-whose-loops-end.md) | Kernels whose loops always end skip wgpu's loop counter: one `unsafe` call in `runtime/gpu` | Accepted |
| [0078](0078-browser-playground.md) | The docs site's playground runs the checker in the browser, as a WASI build of `tools/playground` | Accepted |
| [0079](0079-listen-queue-through-socket2.md) | Listeners ask for the system's longest queue of waiting connections, through `socket2` | Accepted |
| [0080](0080-installing-kavach.md) | An install holds `kavach`, the runtime library and LLVM's tools; the linker is checked, not bundled; archives and install scripts come first | Accepted |
| [0081](0081-packed-executables.md) | `kavach build --compress` packs a program (deflate, or xz with `=max`) behind a small C starter, carried inside `kavach`, that unpacks it once into a cache folder | Accepted |
| [0082](0082-the-json-tape-as-built.md) | A parsed JSON document is one list of words; `Json` arrays and objects point into it and make their list or map only when matched; reads in native code go straight to the tape | Accepted |
| [0083](0083-dependencies-by-path-and-git.md) | Dependencies by `path` and `git` with `kavach.lock`, before a registry; a dependency's modules are named after it, and `unsafe` is allowed per dependency | Accepted |
| [0084](0084-environment-variables-and-granted-capabilities.md) | Capabilities granted per dependency (`can`); `env` values are secrets; `.env` files read by `kavach run`, `kavach test` and built programs, never compiled in | Accepted |
| [0085](0085-function-types-with-errors-and-uses.md) | Function types say their errors and `uses`; a lambda's `try` is its own; built-in methods pass a function's error on; `json.describe(f)` | Accepted |
| [0086](0086-decoding-reports-every-problem.md) | Decoding lists every problem in one `Invalid` error; decoded values are checked against their invariants, a broken one being a problem | Accepted |
| [0087](0087-http-headers-and-server-limits.md) | HTTP headers as a list of `Header`s with `header(name)` and `headers_named(name)`; `http.serve` takes one `limits: ServeLimits` record with safe defaults | Accepted |
| [0088](0088-finishing-before-a-stop.md) | `cli.on_stop(grace)` holds a program's stop so it can finish its work; `http.serve` drains by itself on a stop signal | Accepted |
| [0089](0089-streamed-http-bodies.md) | HTTP request bodies whole up to `buffered_body`, larger ones read with `r.read_body()`; responses can stream a reader, `http.file(path)` | Accepted |
| [0090](0090-forms-read-part-by-part.md) | `http.form(r)` reads a `multipart/form-data` body part by part, each part's data as it comes; `Bytes.index_of` | Accepted |
| [0091](0091-workspaces.md) | `[workspace] members` in the root `kavach.toml`: one lock and one `kavach_modules/` for every member, commands at the root run on each; `dir` for a package in a folder of a repository | Accepted |
| [0092](0092-websockets-and-server-sent-events.md) | A handler returns `http.websocket(r, session)` or `http.events(r, feed)`, a response that takes over the connection; `ring` named directly for the handshake's SHA-1 | Accepted |
| [0093](0093-std-crypto.md) | `std.crypto` from `ring` (already used for TLS) and `argon2` for passwords; no capability | Accepted |
| [0094](0094-secret-results.md) | A function that returns a secret writes `-> secret T`; returning one without it is a compile error | Accepted |
| [0095](0095-shared-update-gives-a-result.md) | `Shared.update` gives the new value; `update_with` gives its function's result beside it | Accepted |
| [0096](0096-unknown-fields-are-refused.md) | Decoding refuses fields the type doesn't have; `unknown: JsonUnknown.Ignore` skips them, as `std.http`'s client does | Accepted |
| [0097](0097-take-gives-taken.md) | `receive.take()` gives `Taken<T> = Item(value: T) \| Closed`, so a sent `none` isn't a closed channel | Accepted |
| [0098](0098-kavach-written-modules-in-their-own-repositories.md) | Modules written in Kavach (`std.http`, `std.grpc`, `std.term`'s prompts) move to repositories of their own; what is native stays | Accepted |
| [0099](0099-client-certificate-names.md) | A client certificate's names with mutual TLS (`conn.client_cert()`, `r.client_cert`), read with `x509-cert` | Accepted |
| [0100](0100-tagged-literals.md) | Tagged literals (`sql"…"`), declared by a package with a pure checker the compiler runs | Accepted |
| [0101](0101-re-exports.md) | Re-exports with `pub use`: a module shows names declared in another, the same declarations | Accepted |
| [0102](0102-unicode-normalization.md) | Unicode normal forms in `std.text` (`text.nfc`, `nfd`, `nfkc`, `nfkd`), from `unicode-normalization` | Accepted |
| [0103](0103-std-compress.md) | `std.compress`: gzip, deflate, zstd, brotli and lz4 through one `encode`/`decode` pair with a `Format` value; decoding stops at `most` (64 MB) | Accepted |
| [0104](0104-windows-through-winit.md) | `std.window`: windows through `winit`; `Window` and `Input` capabilities; events read with `events()` or `poll()`, and a callback form | Accepted |
| [0105](0105-std-process.md) | `std.process`: other programs run with no shell, as trees that end together; `Process` and `Process("name")`; our own system calls in `runtime/process`, allowed `unsafe` | Accepted |
| [0106](0106-drawing-on-the-gpu.md) | Drawing on the GPU: shaders as `gpu vertex fn` and `gpu fragment fn`, a thin `std.gpu`, GPU lists drawn with no copy, pipelines built ahead | Accepted |
| [0107](0107-traces-and-metrics.md) | Traces and metrics in `std.log`: `log.span(name) { … }` blocks, the trace kept per task, `http` joining traces by `traceparent`, `log.count`/`gauge`/`histogram`; blocks given to a call, also for `random.seeded` | Accepted |

## Open decisions

- **Trademark.** A policy for the name "Kavach" and its logo, so forks can't present themselves as the official project.
- **Governance.** When and how to move to an independent foundation.
- **Registry.** Name, hosting and package signing.

## Template

Copy this into a new file named `NNNN-short-title.md` with the next number:

```markdown
# NNNN. Title

- **Status:** Proposed | Accepted | Replaced by NNNN
- **Date:** YYYY-MM-DD

## Context
What problem are we solving? What constraints matter?

## Decision
What we decided, in a few sentences.

## Consequences
What gets better, what gets worse, what we must now do.

## Alternatives considered
Each option we rejected, and why.
```
