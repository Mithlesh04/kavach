# 0098. Modules written in Kavach live in their own repositories

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

A maintainer set a rule (2026-11-12): what is written in Kavach is a package in a repository of its own, as the PostgreSQL client is (`kavach-postgres`); what is native, or a direct use of a third-party library, stays in Kavach. A package can then change and be released without a new Kavach version, and a program fetches only what it uses. Asked about the standard library modules already written in Kavach, the maintainer chose to move them out too (2026-11-13).

## Decision

- **Move out:** `std.http` (HTTP/1.1, HTTP/2, HTTP/3, the server, forms, WebSockets and server-sent events: `stdlib/http/`) becomes the package `http` in its own repository; `std.grpc` (`stdlib/grpc/`) becomes the package `grpc`, which depends on `http`. `std.term`'s prompts and progress bars, written in Kavach, become a package too.
- **Stay native, in Kavach:** what those packages stand on: `std.net` (TCP, UDP, TLS, QUIC), `std.crypto`, `std.base64`, `std.json`, tasks and channels, `std.fs`, `std.time`, `std.cli` (with `on_stop`). A runtime piece a package needs, such as a WebSocket handshake's SHA-1 or the TLS certificate of a peer, is a function of a native module, not part of the package.
- **The spec** keeps the native modules. Spec 14.11 (HTTP) and 14.14 (gRPC) become each package's own documentation, and the spec says where they are. Their conformance tests move with them; the native parts they use keep tests here.
- **Order:** spec and tests first, then the move, in one step per module, with the sessions that depend on them (Rath) told the hash before and after. A new repository is made with a maintainer's go-ahead.

## Consequences

- `use std.http` becomes `use http` with a `[dependencies]` entry; programs and Rath change their `use` lines and manifests.
- The HTTP speed work (`http.serve` on every core, the latency targets of the to-do's speed section) happens in the `http` package, measured from Kavach's benchmarks against a pinned version.
- Kavach's own release no longer ships HTTP or gRPC; a program without them builds and links less.

## Alternatives considered

- **Keep them in std:** one place, but every HTTP change waits for a Kavach release.
- **Move only new modules:** the maintainer chose to move these too.
