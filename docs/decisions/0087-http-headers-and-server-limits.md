# 0087. HTTP headers as a list, and limits on every server

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Rath, a web framework written in Kavach, asked (2026-11-12) for `std.http`'s server to be hardened before it can serve the public: multi-value headers, size limits refused before a body is read, deadlines against slow clients, connection and in-flight limits with load shedding, and more.

Two parts of that were choices about the API:

- `Request` and `Response` held `headers: [String: String]`, so two `set-cookie` lines could not both be sent or read. Joining them with `, ` (which RFC 9110 allows for most headers) breaks cookies, whose values contain commas.
- The limits need a place in `http.serve`'s signature that stays small as more are added.

A maintainer chose (2026-11-12), from the options put to them, both answers below.

## Decision

- **Headers are a list** (spec 14.11): `headers: [Header]`, `Header(name:, value:)` as HTTP/2 and HTTP/3 already used, in the order sent, names in lower case, repeats kept. `r.header(name)` gives the first value, `r.headers_named(name)` all of them, on `Request` and `Response` alike.
- **One `limits:` record** (spec 14.11): `http.serve(listener, handler, limits: ServeLimits(…))`. Every field has a safe default, so a server that sets nothing is protected; a new limit is a new field, not a new parameter.
- The server moves to a module of its own, `std.http.server`; programs still call `http.serve`.

## Consequences

- Cookies and other repeated headers work both ways, as in Starlette and Node.
- Reading one header is a method call, `r.header("content-type")`, instead of a map lookup.
- A server can't be held by a client that sends a byte at a time, uploads more than it allows, or opens more connections than it can serve: such clients get 408, 413, 431 or 503 and their connection closes.

## Alternatives considered

- **A map of lists** (`[String: [String]]`, as Go's `http.Header`): reading one header needs `[0]` or a helper.
- **A map, with cookies apart:** `set-cookie` would stay a special case, and other repeated headers would still be joined.
- **Separate named arguments for each limit:** the signature would grow with every limit.
- **A server builder** (`http.server(listener).max_body(…)`): a new type with many methods, for what is plain data.
