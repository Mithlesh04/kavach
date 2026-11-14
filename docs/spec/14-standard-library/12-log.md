# 14.12 std.log

Logging, written in Kavach (`stdlib/log/log.kav`). A log event is a **record the program defines**, not a string. Its type says that it is an event by implementing `LogEvent`:

```kavach
use std.log

type OrderPlaced: LogEvent {
    order_id: Int
    amount: Decimal
    secret card: String
}

fn main() {
    log.info(OrderPlaced(order_id: 7, amount: 499.00, card: "4242 4242 4242 4242"))
    // INFO OrderPlaced(order_id: 7, amount: 499.00, card: <secret>)
}
```

`LogEvent` is an interface without methods (2.8). Like the standard library's types, it is a name in every module. Only values of types that implement it can be logged: `log.info("order placed")` is a compile error. `std.log` hands each line to the runtime with `log_write`, which only the standard library may call.

| Function | Level | For |
|---|---|---|
| `log.debug(event)` | `debug` | Details for finding a problem |
| `log.info(event)` | `info` | What the program did |
| `log.warn(event)` | `warn` | Something unexpected that the program handled |
| `log.error(event)` | `error` | Something that failed |

## Levels

The levels, from lowest to highest, are `debug`, `info`, `warn` and `error`. Events below the lowest level written are left out. That level is `info` unless `kavach.toml` sets another (below).

## Where events go

Each event is one line: the level in capitals, a space, and the event in its printed form (1.11). So a `secret` field is written as `<secret>` (2.11), and a type can choose its own form by implementing `Display`.

```kavach
type Retrying: LogEvent {
    attempt: Int
}

log.warn(Retrying(attempt: 2))      // WARN Retrying(attempt: 2)
```

Lines go to standard output, and to a file when `kavach.toml` names one. The program's owner chooses, not its code:

```toml
[log]
level = "debug"              # the lowest level written (default "info")
console = false              # standard output (default true)
file = "logs/app.log"        # also, or only, here
max_bytes = 10_000_000       # then start a new file (default 10 MB)
keep = 5                     # older files kept (default 5)
```

A program's parts can have their own lowest levels, by the module an event type is declared in, or by the event type itself. The event type's setting wins over its module's, which wins over `level`:

```toml
[log]
level = "warn"

[log.levels]
billing = "debug"            # events declared in module `billing`
Retrying = "error"           # this event type, wherever it is declared
```

On standard output, when it is a terminal, the level is in color: `DEBUG` gray, `INFO` green, `WARN` yellow, `ERROR` bold red. Output to a file or a pipe has no colors, and neither does a terminal when the environment variable `NO_COLOR` is set ([no-color.org](https://no-color.org)).

A file line starts with the time, in UTC: `2026-11-02T10:14:07Z INFO OrderPlaced(order_id: 7, …)`. Its folder is made when needed. When a line would make the file larger than `max_bytes`, the file is renamed `app.log.1` (an older `app.log.1` becomes `app.log.2`, and so on, up to `keep`; the oldest is deleted) and a new `app.log` is started. The path is relative to the folder of `kavach.toml`. The settings are read when the program is compiled, as for agents (8.5).

Events can also go to an OpenTelemetry collector, which passes them on to the tools a company already runs:

```toml
[log.otlp]
endpoint = "http://localhost:4318"   # OTLP over HTTP, with JSON
service = "shop"                     # its service.name (default: the package's name)
```

Each event written also goes there as a log record: its level as the severity, its printed form as the body, and its type's name as the attribute `event.name`. A thread of its own sends them in batches, at least once a second, so a slow collector never slows the program. When more than 10,000 events wait, new ones are dropped, and a record `LogEventsDropped(count: …)` says how many. When the program ends, the events still waiting are sent, for at most 2 seconds. Sampling comes later ([platform plan](../../vision/platform.md#logging-and-remote-pipelines)).

## No capability

Logging needs no capability (8.1, [decision 0023](../../decisions/0023-logging-needs-no-capability.md)). Where events go is chosen by the program's owner, not by the code that writes them, and the code can't read them back. A function that only logs needs no `uses`:

```kavach
pub fn refund(order_id: Int) -> Int {
    log.info(Refunded(order_id: order_id))
    order_id
}
```

## Traces

A trace follows one piece of work, such as a request, through the program and the services it calls ([decision 0107](../../decisions/0107-traces-and-metrics.md)). It is made of spans: `log.span(name) { … }` runs its block as one span (4.8), timed from its start to its end.

```kavach
fn checkout(cart: Cart) -> Receipt ! ShopError {
    log.span("checkout") {
        const paid = try pay(cart)
        log.info(Paid(amount: paid.amount))     // carries the span's trace and span ids
        Receipt(id: paid.id)
    }
}
```

- A span inside another is its child. The first span of a task starts a new trace, unless the task already has one.
- A task keeps the trace of the code that spawned it, so the spans of tasks a request spawns are part of the request's trace.
- Each event written inside a span carries the span's trace and span ids to the collector.
- A span whose block fails (5.2) is marked as an error. One cut short by a panic is not sent: the panic says what happened.
- `log.span(name, from: header) { … }` continues the trace that a W3C `traceparent` header names, when the code isn't in a span already. `log.traceparent()` gives the current span's header, or `none` outside a span. Together they carry a trace over any connection: a queue, gRPC metadata, a file.
- `http.serve` runs each request's handler in a span named by its method (`GET`). When the request has a W3C `traceparent` header, the span continues that trace. The span's name never holds the path, which can hold ids and tokens (`/users/7/reset/<token>`); code that knows the route names the span with it, `log.name_span("GET /orders/\{id}")`. `http.request` and the other calls send the current span's `traceparent`. So traces join across services with no code (14.11).
- `log.name_span(name)` renames the current span, once the code knows what it is doing; outside a span it does nothing. A name should be one of a few, such as a route's pattern, not text with ids in it.
- `log.trace_id()` gives the current trace's id, 32 hexadecimal digits, or `none` outside a span: for an error page or a header in a reply.
- With `[log.otlp]` set, each span goes to the collector when it ends, batched as events are (`POST <endpoint>/v1/traces`). Without it, spans are sent nowhere.

## Metrics

Numbers a program counts or measures go to the collector every 10 seconds as OpenTelemetry metrics (`POST <endpoint>/v1/metrics`), when `[log.otlp]` is set:

| Function | Sends |
|---|---|
| `log.count(name, n)` | A sum: `n` added to the counter `name`. `n` is 0 or more (a `require`) |
| `log.gauge(name, value)` | The latest `value` of `name`, such as a queue's length |
| `log.histogram(name, value)` | How the `value`s of `name` spread, such as each request's time in milliseconds: how many, their sum, the smallest, the largest, and how many fall in each bucket (OpenTelemetry's default bounds: 0, 5, 10, 25, 50, 75, 100, 250, 500, 750, 1000, 2500, 5000, 7500, 10000) |

```kavach
log.count("orders_placed", 1)
log.histogram("checkout_ms", Float(elapsed))
```

- `name` is plain text; `n` is an `Int`, `value` a `Float`.
- Each takes `attributes: [String: String]` too, empty unless given: `log.histogram("checkout.duration_ms", ms, attributes: ["http.route": "/orders/\{id}", "http.response.status_code": "200"])`. Each name and set of attributes is a series of its own, whatever order the attributes were written in. Give attributes with few values (a route's pattern, a method, a status), never ids: each new value makes a new series to keep and send. Put the unit in the name (`_ms`): the buckets above are for milliseconds, and OpenTelemetry's own `http.server.request.duration` is in seconds.
- Without `[log.otlp]`, metrics are kept nowhere and cost almost nothing.
- Like events, spans and metrics need no capability.

