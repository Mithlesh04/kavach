# 0092. WebSockets and server-sent events: a response that takes over

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

A handler is `fn(Request) -> Response`: it answers and is done. A WebSocket (RFC 6455) and a stream of server-sent events (`text/event-stream`) answer once, then keep the connection for as long as the client stays. Rath, a web framework written in Kavach, asked (2026-11-12) for both.

A maintainer chose (2026-11-12) the first of the options put to them.

## Decision

- **A response that takes over** (spec 14.11): a handler returns `http.websocket(r, session)` or `http.events(r, feed)`. The server sends the response's head (`101 Switching Protocols`, or `200` with `text/event-stream`), then runs `session` or `feed` with the connection, on the connection's own task. Handlers keep one signature.
- **WebSockets:** `ws.receive() -> WsMessage?` (`Text` or `Binary`), `ws.send_text`, `ws.send_bytes`, `ws.close()`; pings are answered by themselves, and a message larger than `max_body_bytes` closes the socket with 1009. A client opens one with `http.websocket_connect(url)`.
- **Server-sent events:** `out.send(data, event:, id:)`.
- **The handshake's SHA-1** comes from `ring`, already used for TLS ([decision 0021](0021-tls-through-rustls.md)); `runtime/net` now names it directly. Nothing is hand-written: the framing is plain Kavach, and only the accept value, the client's key and its masks come from the runtime.

## Consequences

- Chat, live dashboards and notifications need no other library.
- A WebSocket or event stream holds its connection and a task, not a request: it counts against `max_connections`, not `max_in_flight`.
- Over HTTP/2 neither is offered yet: a request for one there gets 501.

## Alternatives considered

- **A second kind of handler** (`fn(Request, WebSocket)`): clearer types, but two handler kinds through every framework.
- **Channels** (`(Response, Sender, Receiver)`): fits tasks, but the connection's life is harder to follow.
