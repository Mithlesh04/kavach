# 14. Standard library

Conformance tests for spec chapter [14. Standard library](../../../docs/spec/14-standard-library/README.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 14.2 | [number-methods.kav](number-methods.kav) | Numbers have abs, min, max and clamp. | runs |
| 14.2 | [text-methods.kav](text-methods.kav) | Text has replace, lines, chars, index_of and slice; positions count characters. | runs |
| 14.2 | [text-split.kav](text-split.kav) | split returns the parts between separators, empty parts included. | runs |
| 14.2 | [list-methods.kav](list-methods.kav) | Lists have join, sort, sort_by, reverse, take, skip, find, count, index_of, min and max. | runs |
| 14.2 | [sets-hold-each-value-once.kav](sets-hold-each-value-once.kav) | A set holds each value once and keeps insertion order. | runs |
| 14.2 | [set-changes-need-a-var.kav](set-changes-need-a-var.kav) | Changing a set with `add` or `remove` needs a `let`. | compile error |
| 14.1 | [files-need-fs.kav](files-need-fs.kav) | Calling std.fs needs the Fs capability. | compile error |
| 14.1 | [unknown-modules-are-errors.kav](unknown-modules-are-errors.kav) | Only the modules of chapter 14 exist in `std`. | compile error |
| 14.1 | [std-types-are-named-by-use.kav](std-types-are-named-by-use.kav) | `use std.time.{Date}` names the type itself, which makes values and is checked like any type. | compile error |
| 14.1 | [runtime-functions-are-not-named-alone.kav](runtime-functions-are-not-named-alone.kav) | `use std.fs.{read_text}` is a compile error: a function the runtime provides is called through its module. | compile error |
| 14.3 | [files-read-and-write.kav](files-read-and-write.kav) | std.fs reads, writes, appends, lists and removes files. | runs |
| 14.3 | [missing-files-are-errors.kav](missing-files-are-errors.kav) | A file operation that fails returns an error value, not a crash: an `FsError`. | runs |
| 14.3 | [files-hold-bytes.kav](files-hold-bytes.kav) | `fs.write_bytes` writes any bytes atomically; `fs.read_bytes` reads them back. | runs |
| 14.3 | [files-are-read-as-the-loop-goes.kav](files-are-read-as-the-loop-goes.kav) | `fs.lines` and `fs.chunks` read a file as the loop goes; opening a missing file is an `FsError`. | runs |
| 2.13, 14.3 | [big-chunks-read-as-numbers.kav](big-chunks-read-as-numbers.kav) | `as_list` on a big chunk of a file gives its numbers, whether or not anything else holds the chunk, and a chunk read again keeps its bytes. | runs |
| 14.3 | [big-chunks-read-the-whole-file-in-order.kav](big-chunks-read-the-whole-file-in-order.kav) | Pieces of 1 MB or more come as any others do, and the file can go when the loop stops early. | runs |
| 14.3 | [read-errors-go-to-the-loop-header.kav](read-errors-go-to-the-loop-header.kav) | A read that fails during a loop over `fs.lines` goes to the header's `try` (the error is returned) or `catch` (its handler runs). | runs |
| 14.3, 10.2 | [fs-errors-can-be-matched.kav](fs-errors-can-be-matched.kav) | An `FsError` is a union that code can `match` on, naming its variants through the module (`fs.NotFound`) or alone. | runs |
| 14.4 | [clocks-move-forward.kav](clocks-move-forward.kav) | time.monotonic_ms never goes back, and time.sleep_ms waits. | runs |
| 14.4 | [tests-freeze-the-clock.kav](tests-freeze-the-clock.kav) | A `test` block freezes the clock with `time.freeze`; `sleep` moves it on at once. | kavach test |
| 14.4 | [freeze-is-only-for-tests.kav](freeze-is-only-for-tests.kav) | `time.freeze` outside a `test` block is a compile error. | compile error |
| 14.5 | [math-functions.kav](math-functions.kav) | std.math has sqrt, pow, floor, ceil, round, pi and e. | runs |
| 14.6 | [cli-parse-reads-options.kav](cli-parse-reads-options.kav) | cli.parse reads options into a record; fields with defaults are optional. | runs |
| 14.6 | [cli-parse-reports-missing-options.kav](cli-parse-reports-missing-options.kav) | A field with no default is a required option; cli.parse says which is missing, then the usage. | runs |
| 14.6 | [cli-parse-reads-a-command.kav](cli-parse-reads-a-command.kav) | cli.parse into a union reads the first argument as the command and the rest as its options. | runs |
| 14.6 | [cli-parse-reports-unknown-commands.kav](cli-parse-reports-unknown-commands.kav) | An unknown command is an error: the problem, then the usage of every command. | runs |
| 14.6 | [cli-parse-command-errors-show-its-usage.kav](cli-parse-command-errors-show-its-usage.kav) | A problem with a command's options gives that command's usage. | runs |
| 14.6 | [exit-ends-the-program-while-tasks-wait.kav](exit-ends-the-program-while-tasks-wait.kav) | `cli.exit(code)` ends the program at once with `code`, even while other tasks wait. | runs |
| 14.4 | [intervals-count-rounds.kav](intervals-count-rounds.kav) | A for loop over time.every_ms counts rounds, each starting ms after the one before. | runs |
| 14.4 | [durations-are-written-with-units.kav](durations-are-written-with-units.kav) | A whole number followed by `ms`, `seconds`, `minutes`, `hours` or `days` is a `Duration`; `time.sleep` and `time.every` take one. | runs |
| 14.4 | [durations-have-no-months.kav](durations-have-no-months.kav) | A whole number takes only the units of a `Duration`: `2.months` is a compile error, since a month has no fixed length. | compile error |
| 14.4 | [timers-call-a-function-later.kav](timers-call-a-function-later.kav) | `time.after(d, f)` is a task that calls `f` after `d`, cancelled when the function that made it returns. | runs |
| 14.4 | [zones-show-local-time.kav](zones-show-local-time.kav) | A `Zone` from `time.zone(name)` gives the date and time its clocks show at a moment, daylight saving time included. | runs |
| 14.4 | [zones-move-a-skipped-time-later.kav](zones-move-a-skipped-time-later.kav) | A time of day the clocks skipped when daylight saving time began moves later by the gap, as RFC 5545 does; `resolve` gives both moments. | runs |
| 14.4 | [zones-take-a-repeated-time-earlier.kav](zones-take-a-repeated-time-earlier.kav) | A time of day the clocks showed twice when daylight saving time ended is its earlier moment, as RFC 5545 does; `resolve` gives both. | runs |
| 14.4 | [zones-unknown-name-fails.kav](zones-unknown-name-fails.kav) | `time.zone(name)` fails with `UnknownZone(name)` for a name the IANA database doesn't have. | runs |
| 14.4 | [zones-can-be-fixed-offsets.kav](zones-can-be-fixed-offsets.kav) | `time.zone` also takes a fixed offset from UTC; one that can't be fails with `UnknownZone`. | runs |
| 14.4 | [zones-write-moments-with-their-offset.kav](zones-write-moments-with-their-offset.kav) | `z.iso(ms)` writes a moment with its offset, and `parse_moment` reads it back. | runs |
| 14.4 | [times-are-read-from-text.kav](times-are-read-from-text.kav) | `parse_time` and `parse_datetime` read what `iso` writes; anything else fails with `Invalid`. | runs |
| 14.4 | [moments-are-read-only-with-an-offset.kav](moments-are-read-only-with-an-offset.kav) | `parse_moment` needs `Z` or an offset: a time with no zone is no moment. | runs |
| 14.4 | [dates-follow-the-gregorian-calendar.kav](dates-follow-the-gregorian-calendar.kav) | Dates follow the Gregorian calendar with months 1 to 12; adding a month keeps the day or gives the month's last; a moment becomes a `DateTime` in UTC. | runs |
| 14.4, 8.1 | [reading-the-clock-needs-no-capability.kav](reading-the-clock-needs-no-capability.kav) | Reading the clock needs no capability, even in a `pub` function. | runs |
| 14.4 | [contracts-cannot-read-the-clock.kav](contracts-cannot-read-the-clock.kav) | A contract may not read the clock. | compile error |
| 14.4 | [impossible-dates-cannot-be-made.kav](impossible-dates-cannot-be-made.kav) | A `Date` that can't exist breaks its invariant. | panic |
| 14.8 | [regex-methods.kav](regex-methods.kav) | A Regex finds, captures, replaces and splits text. | runs |
| 14.8 | [regex-is-linear-time.kav](regex-is-linear-time.kav) | regex.new matches in time that grows with the text only, whatever the pattern. | runs |
| 14.8 | [lookaround-needs-advanced.kav](lookaround-needs-advanced.kav) | Lookaround and backreferences need regex.advanced. | runs |
| 14.8 | [advanced-regex-gives-up.kav](advanced-regex-gives-up.kav) | An advanced pattern that needs too much work on some text panics instead of hanging. | panic |
| 14.8 | [invalid-patterns-are-errors.kav](invalid-patterns-are-errors.kav) | An invalid pattern is an error value, not a crash. | runs |
| 14.5 | [math-module.kav](math-module.kav) | `std.math` has the runtime's `sqrt` and `pi` and, written in Kavach, `gcd`, `lcm`, `factorial`, `is_prime` and `mean`. | runs |
| 14.9 | [text-module.kav](text-module.kav) | `std.text` pads, splits into words, capitalizes, truncates and finds blank text. | runs |
| 14.9 | [text-finds-numbers.kav](text-finds-numbers.kav) | `std.text` finds numbers inside other text. | runs |
| 14.2 | [float-near.kav](float-near.kav) | `x.near(y)` is true for floats equal but for rounding, never for `NaN`. | runs |
| 14.2 | [sort-puts-nan-last.kav](sort-puts-nan-last.kav) | `NaN` sorts after every number, so a list holding one still sorts. | runs |
| 14.2 | [sort-by-needs-an-ordered-key.kav](sort-by-needs-an-ordered-key.kav) | `sort_by`'s key must have an order: a record key is a compile error, not a list left unsorted. | compile error |
| 14.2 | [floor-division-and-decimal-rounding.kav](floor-division-and-decimal-rounding.kav) | `x.div_floor(n)` rounds down and `x.mod(n)` has the sign of `n`; `d.round(places)` rounds a half away from zero and `d.round_even(places)` to the even digit. | runs |
| 14.13, 14.6 | [term-colors-are-off-when-not-a-terminal.kav](term-colors-are-off-when-not-a-terminal.kav) | std.term colors and styles text only when standard output is a terminal. | runs |
| 14.6 | [io-colors-need-a-terminal.kav](io-colors-need-a-terminal.kav) | `io.colors()` is `false` when standard output is not a terminal. | runs |
| 14.13 | [term-progress-draws-only-on-a-terminal.kav](term-progress-draws-only-on-a-terminal.kav) | A progress bar prints nothing when output is not a terminal. | runs |
| 14.13, 8.7 | [term-secrets-cant-be-printed.kav](term-secrets-cant-be-printed.kav) | `term.prompt_secret` gives a `Secret` whose `text` can't be printed. | compile error |
| 14.9 | [collections-module.kav](collections-module.kav) | `std.collections` makes chunks, zips two lists, keeps each item once, counts items and flattens lists. | runs |
| 14.10 | [tcp-connections-carry-bytes.kav](tcp-connections-carry-bytes.kav) | TCP connections carry `Bytes` both ways; `read` gives `none` once the other side closed. | runs |
| 14.10 | [connections-need-net.kav](connections-need-net.kav) | Using a `Connection` needs `Net`. | compile error |
| 14.10 | [udp-carries-datagrams.kav](udp-carries-datagrams.kav) | `net.bind` makes a UDP socket; `send` and `receive` carry datagrams, with the sender's address. | runs |
| 14.10 | [net-timeouts-set-deadlines.kav](net-timeouts-set-deadlines.kav) | `.timeout(d)` on a network wait sets its deadline; passing it fails with `NetError.TimedOut`. | runs |
| 14.11 | [hpack-decodes-rfc-examples.kav](hpack-decodes-rfc-examples.kav) | `std.http.hpack` decodes RFC 7541's examples (Huffman codes, the dynamic table, dropping old entries), and reads back what `encode` writes. | runs |
| 14.11 | [qpack-reads-static-and-literal-fields.kav](qpack-reads-static-and-literal-fields.kav) | `std.http.h3` decodes QPACK blocks with static entries, literals and Huffman codes (RFC 9204 B.1), reads back what `encode` writes, and refuses blocks that need a dynamic table. | runs |
| 14.11 | [http-serves-and-fetches.kav](http-serves-and-fetches.kav) | `std.http` serves from a handler and fetches status, headers and body; other URLs than `http://` and `https://` give `BadUrl`. | runs |
| 14.11 | [http-servers-keep-connections-open.kav](http-servers-keep-connections-open.kav) | `http.serve` keeps an HTTP/1.1 connection open for the next request, after a chunked body too, and closes it when asked. | runs |
| 14.11 | [http-sessions-reuse-one-connection.kav](http-sessions-reuse-one-connection.kav) | A session's requests all go over one connection, each waiting for its response. | runs |
| 14.11 | [http-requests-take-any-method-and-headers.kav](http-requests-take-any-method-and-headers.kav) | `http.request(method, url, headers, body)` and `session.request(…)` send any method, the headers given and a body. | runs |
| 14.11, 14.10 | [http-requests-know-who-sent-them.kav](http-requests-know-who-sent-them.kav) | A served `Request` has `peer`, the address of whoever opened the connection, as `conn.peer()` gives it. | runs |
| 14.11 | [http-json-shares-types.kav](http-json-shares-types.kav) | A server and its client share types through `std.http`'s JSON calls; a status other than 2xx is `Status`, a body of the wrong shape `BadJson`. | runs |
| 14.2 | [float-bits-round-trip.kav](float-bits-round-trip.kav) | `x.bits()` gives a `Float`'s IEEE 754 bits; `Float.from_bits(n)` gives it back. | runs |
| 14.14 | [grpc-messages-are-protocol-buffers.kav](grpc-messages-are-protocol-buffers.kav) | `grpc.encode<T>` writes Protocol Buffers with field `n` the type's `n`th field; `grpc.decode<T>` reads them back. | runs |
| 14.14, 2.11 | [grpc-refuses-secret-fields.kav](grpc-refuses-secret-fields.kav) | A value holding a `secret` field can't become a gRPC message: encoding refuses at run time. | panic |
| 14.14 | [grpc-unions-are-oneof-messages.kav](grpc-unions-are-oneof-messages.kav) | A union field travels as a wrapper message holding a oneof; one that never arrived fails to decode. | runs |
| 14.14, 2.12 | [grpc-maps-use-entry-messages.kav](grpc-maps-use-entry-messages.kav) | A map field travels as repeated entry messages, and `json.schema` gives maps an open object. | runs |
| 14.14 | [grpc-streams-deliver-each-message.kav](grpc-streams-deliver-each-message.kav) | `grpc.reply_stream` sends each value as its own message, and `grpc.each` runs its function per message, failing with `Status` as `call` does. | runs |
| 14.14 | [grpc-client-streams-send-every-message.kav](grpc-client-streams-send-every-message.kav) | `grpc.call_stream` sends each request as its own message; the handler reads them all with `grpc.read_all`, `call.message` is the first. | runs |
| 14.14 | [grpc-talks-answer-as-messages-arrive.kav](grpc-talks-answer-as-messages-arrive.kav) | On a method in `serve_talks`'s list the handler answers each message as it arrives, and `talk_with` sends what its function gives next, so each side answers the other's last message. | runs |
| 14.14 | [grpc-two-way-streams.kav](grpc-two-way-streams.kav) | `grpc.each_stream` sends every request, then runs its function per message of the answer, failing with `Status` as `call` does. | runs |
| 14.3 | [durable-writes-force-the-disk.kav](durable-writes-force-the-disk.kav) | `fs.write_durable` replaces the file atomically and forces it to the disk; file scopes hold for it. | runs |
| 14.3 | [fs-info-and-permissions.kav](fs-info-and-permissions.kav) | `fs.info` gives a path's kind, size and permissions; `set_read_only` and `set_permissions` change them; `copy`, `rename` and `remove_all` move and delete; a mode past `0o7777` fails. | runs |
| 14.10 | [tls-has-no-insecure-switch.kav](tls-has-no-insecure-switch.kav) | TLS verification can't be turned off: there is no argument that skips certificate checks. | compile error |
| 14.14 | [grpc-calls-from-types.kav](grpc-calls-from-types.kav) | `grpc.call<Req, Res>` calls a method with typed messages; `grpc.serve` answers; a status other than OK fails with `Status`. | runs |
| 14.11 | [http2-serves-and-fetches.kav](http2-serves-and-fetches.kav) | `http.serve` answers HTTP/2 from a client that starts with its preface; `h2.request` sends a large body with flow control; HTTP/1.1 still works. | runs |
| 14.11 | [http-reads-chunked-bodies.kav](http-reads-chunked-bodies.kav) | Requests and responses sent in chunks are read whole; a chunked response cut off fails with `BadMessage`. | runs |
| 14.11 | [http-serves-many-clients-on-every-core.kav](http-serves-many-clients-on-every-core.kav) | `http.serve` answers each connection in a `spawn cores` task: 8 clients at once get the right answers, and handlers share only a `Shared` count. | runs |
| 14.11 | [http-headers-keep-repeats.kav](http-headers-keep-repeats.kav) | Headers are a list in the order sent, repeats kept; `header(name)` gives the first, `headers_named(name)` all. | runs |
| 14.11 | [http-serve-limits-refuse-what-is-too-much.kav](http-serve-limits-refuse-what-is-too-much.kav) | `http.serve` refuses a body past `max_body_bytes` (413), headers past `max_header_bytes` (431) or too slow (408), answers `expect: 100-continue`, and gives 503 with `retry-after` past `max_in_flight` or `max_connections`. | runs |
| 14.11 | [http-handlers-stop-when-the-client-leaves.kav](http-handlers-stop-when-the-client-leaves.kav) | A handler's task is cancelled when its client closes the connection; a handler that panics gets 500. | runs |
| 14.11 | [http-large-bodies-stream.kav](http-large-bodies-stream.kav) | A body past `buffered_body` is read with `read_body` and the connection goes on; a response sends a `fs.chunks` reader as it is read, with its length (`http.file`) or in chunks, over HTTP/1.1 and HTTP/2. | runs |
| 14.11 | [http-forms-read-part-by-part.kav](http-forms-read-part-by-part.kav) | `http.form(r)` reads a `multipart/form-data` body part by part, from a body read whole or read with `read_body`; a part not read is skipped. | runs |
| 14.11 | [http-websockets-and-events.kav](http-websockets-and-events.kav) | `http.websocket` and `http.events` take over the connection: text and binary messages both ways (a client from `http.websocket_connect`), and `event:`/`data:` lines. | runs |
| 14.19 | [crypto-hashes-signs-and-seals.kav](crypto-hashes-signs-and-seals.kav) | `std.crypto` hashes, checks Argon2id passwords, signs and verifies with Ed25519 and P-256, and seals with AES-GCM so a changed copy fails with `Tampered`; no capability. | runs |
| 14.19 | [crypto-checks-rsa-signatures.kav](crypto-checks-rsa-signatures.kav) | `crypto.rsa_verify` checks an RSA signature with SHA-256 (RS256, or PS256 with `pss: true`) from a public key's modulus and exponent. | runs |
| 14.20 | [base64-encodes-and-decodes.kav](base64-encodes-and-decodes.kav) | `base64.encode` writes bytes as base64, standard or URL-safe, with or without padding, and `base64.decode` reads it back strictly. | runs |
| 5.8, 14.6 | [cli-on-stop-needs-no-capability.kav](cli-on-stop-needs-no-capability.kav) | `cli.on_stop(grace)` gives a hold that isn't asked until a stop signal comes; taking and letting go of one needs no capability. | runs |
| 14.12 | [log-events-are-records.kav](log-events-are-records.kav) | A log event is a value of a type that implements `LogEvent`; the console shows its level and its printed form. | runs |
| 14.12 | [log-hides-secrets.kav](log-hides-secrets.kav) | A secret field of a log event is written as `<secret>`. | runs |
| 14.12 | [log-debug-is-off-by-default.kav](log-debug-is-off-by-default.kav) | Events below level `info` are left out unless the level is set lower. | runs |
| 14.12 | [log-needs-no-capability.kav](log-needs-no-capability.kav) | Logging needs no capability, so a function that only logs needs no `uses`. | runs |
| 14.12 | [log-takes-only-events.kav](log-takes-only-events.kav) | Only values of types that implement `LogEvent` can be logged; text cannot. | compile error |
| 14.12 | [log-write-is-only-for-std-log.kav](log-write-is-only-for-std-log.kav) | `log_write`, how `std.log` hands lines to the runtime, can't be called by a program. | compile error |
| 14.15 | [random-numbers-stay-in-their-range.kav](random-numbers-stay-in-their-range.kav) | `random.int` stays in `from` up to, not including, `until`; `random.float` in 0.0 up to 1.0. | runs |
| 14.15 | [random-shuffles-keep-every-item.kav](random-shuffles-keep-every-item.kav) | `random.shuffle` gives the same items, each as often as before. | runs |
| 14.15 | [random-pick-from-nothing-is-none.kav](random-pick-from-nothing-is-none.kav) | `random.pick` gives one of the items, or `none` when there are none. | runs |
| 14.15 | [random-bytes-have-the-asked-length.kav](random-bytes-have-the-asked-length.kav) | `random.bytes(n)` gives exactly `n` bytes. | runs |
| 14.15 | [random-ranges-must-hold-a-number.kav](random-ranges-must-hold-a-number.kav) | `random.int` with an empty range is a contract failure. | panic |
| 14.15 | [random-bytes-are-at-most-64-kib.kav](random-bytes-are-at-most-64-kib.kav) | `random.bytes(n)` takes at most 65,536 bytes at a time. | panic |
| 14.15 | [random-needs-the-random-capability.kav](random-needs-the-random-capability.kav) | Every `std.random` function needs the `Random` capability. | compile error |
| 14.15 | [random-bits-is-only-for-std-random.kav](random-bits-is-only-for-std-random.kav) | `random_bits`, how `std.random` gets bits from the runtime, can't be called by a program. | compile error |
| 14.16 | [call-key-is-none-outside-a-call.kav](call-key-is-none-outside-a-call.kav) | `agents.call_key()` is `none` outside a capability call of a durable agent's handler. | runs |
| 14.16 | [agents-cant-answer-approvals.kav](agents-cant-answer-approvals.kav) | An agent can't answer approvals: `Approvals` may never be listed in an agent's `can`. | compile error |
| 14.16 | [answering-approvals-needs-approvals.kav](answering-approvals-needs-approvals.kav) | Listing and answering approval requests in code needs the capability `Approvals`. | compile error |
| 14.17 | [clipboard-needs-the-clipboard-capability.kav](clipboard-needs-the-clipboard-capability.kav) | Reading or writing the clipboard needs the capability `Clipboard`: what a person copied is private. | compile error |
| 14.18 | [serial-ports-need-the-serial-capability.kav](serial-ports-need-the-serial-capability.kav) | Listing, opening, reading and writing serial ports needs the capability `Serial`. | compile error |
