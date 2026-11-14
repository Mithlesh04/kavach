# runtime/net (`kavach-runtime-net`)

Networking that both the interpreter and native code use, so the two behave exactly alike. It is a crate of its own, apart from [`core/`](../core/README.md): the code a program uses without networking never shares a compiled unit with TLS, so the linker leaves TLS out of programs that don't use it ([decision 0024](../../docs/decisions/0024-pay-only-for-what-you-use.md)).

| File | Job |
|---|---|
| `src/lib.rs` | The crate's two modules |
| `src/net.rs` | TCP and UDP (spec 14.10): listening, connecting, reading and writing, sending and receiving datagrams, each tried without blocking so the caller can let other tasks run; TLS clients and servers, and mutual TLS, in `src/net/tls.rs` (decision 0021); QUIC client connections and their streams in `src/net/quic.rs`, with `quinn-proto` (decision 0022); the readiness queue that wakes a task when its socket is ready in `src/net/poller.rs`, through `mio` (decision 0046), each wait for reading or writing woken only by its own kind of event (`src/net/ready.rs`); listeners queue as many waiting connections as the system allows, through `socket2` (decision 0079; tests in `src/net/tests.rs`) |
| `src/ai.rs` | `ask<T>` (spec 8.4): the request to the model with the answer's schema, and reading the reply (HTTP in `src/ai/http.rs`, tests in `src/ai/tests.rs`) |
| `src/otlp.rs` | OpenTelemetry export of log events and spans (spec 14.12): OTLP JSON over HTTP, in batches from a thread of its own, the rest sent at the end (tests in the file) |
| `src/otlp_traces.rs` | Spans as OTLP JSON (`/v1/traces`, decision 0107) |
| `src/otlp_metrics.rs` | `log.count`, `log.gauge` and `log.histogram`, kept and sent as OTLP metrics every 10 seconds (`/v1/metrics`) |
