# 0022. HTTP/2 written in Kavach; HTTP/3 over quinn-proto

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

`std.http` (spec 14.11) speaks HTTP/1.1. It is written in Kavach on top of `std.net`. The platform plan asks for HTTP/2 and HTTP/3 clients and servers too. gRPC, planned for later, runs only over HTTP/2.

- **HTTP/2** has binary frames, many requests on one connection (streams), flow control and header compression (HPACK). Browsers and most servers use it only over TLS, and choose it during the handshake (ALPN).
- **HTTP/3** is HTTP over QUIC: a transport on UDP with its own TLS 1.3, loss recovery and congestion control. Its header compression is QPACK.

Tasks take turns (decision 0020), so a task waiting for the network must let others run. The common Rust libraries (`h2`, `h3`, `quinn`) are built on an async runtime (tokio).

## Decision

- **HTTP/2 is written in Kavach**, in `stdlib/http/`, like HTTP/1.1: frames, streams, flow control and HPACK.
  - **HPACK decoding** is complete: static and dynamic tables, and Huffman codes. **Encoding** uses literal headers without indexing and without Huffman codes. RFC 7541 allows this, and it can't go wrong.
  - The runtime adds only **ALPN** to TLS. `net.connect_tls` offers protocols, and the connection tells which one the server chose.
  - `https://` requests offer `h2` and `http/1.1` and use what the server picks.
  - `http.serve` answers HTTP/2 without TLS too, when a client starts with the HTTP/2 preface ("prior knowledge"). gRPC between services uses this.
- **HTTP/3 uses `quinn-proto`** for QUIC, in `runtime/core`.
  - `quinn-proto` is a state machine with no I/O and no async runtime. Our UDP sockets and the scheduler's try-and-wait loop drive it, as for TCP.
  - It uses rustls, already in the runtime (decision 0021).
  - HTTP/3 frames and QPACK are written in Kavach. We advertise a QPACK table size of 0, so a server never sends dynamic-table references.
- Each protocol keeps the same `Request`, `Response` and `HttpError` types. A program doesn't change when the protocol does.

## Consequences

- One HTTP stack, readable Kavach code, the same in the interpreter and native code. gRPC can be built on it.
- Kavach code is slower than a tuned Rust library at first. Header compression and framing run in the program's own code, so speed work on the compiler speeds them up too.
- HTTP/3 adds `quinn-proto` (and its dependencies) to the runtime: a larger executable.
- Still to do after the first version: server push (deprecated in practice; never sent), stream priorities (ignored, as RFC 9113 allows), HPACK Huffman encoding, and QPACK's dynamic table.

## Alternatives considered

- **The `h2` and `h3`/`quinn` crates:** complete and fast, but they need tokio. Its scheduler would run beside ours, and waits would cross between the two. Rejected for now; a tokio-free `h2` would change this.
- **`hyper` or libcurl:** the same async runtime problem (hyper), or a C library to build and ship on every platform (curl).
- **QUIC written in Kavach:** loss recovery and congestion control are too large and too subtle to write before the protocol is in use. `quinn-proto` is maintained, tested and needs no runtime.
- **HTTP/2 only over TLS:** simpler, but services talking gRPC inside one network often use HTTP/2 without TLS. We keep both.
