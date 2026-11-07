# 0023. Logging needs no capability

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

`std.log` (spec 14.12) writes log events: records the program defines, such as `OrderPlaced`. A function that touches the outside world must say so with `uses` (spec 8.1). `print` needs `Console`. `debug` needs no capability, but it is removed from release builds.

Logging is wanted almost everywhere, including deep inside code that is otherwise pure. The platform plan sets where events go (the console, files, OpenTelemetry, Kavach Link) in `kavach.toml`, not in code.

## Decision

- **Calling `log.debug`, `log.info`, `log.warn` or `log.error` needs no capability.** A function that only logs still has no `uses` clause.
- **The program's owner chooses the sinks**, in `kavach.toml` (and the console by default). The code that writes an event can't choose where it goes, and can't read events back.
- **Secrets stay out.** An event is written in its printed form, where a `secret` field shows as `<secret>` (spec 2.11).

```kavach
pub fn refund(order_id: Int) -> Int {      // no `uses` needed
    log.info(Refunded(order_id: order_id))
    order_id
}
```

## Consequences

- Signatures keep showing what matters: `uses Fs`, `uses Net`, `uses Payments`. They are not filled with a capability that almost every function would need.
- A function without `uses` can now write log lines. It still can't affect anything the program can see: nothing it does comes back to it.
- An agent (spec 8) can log without being granted anything. Its events go to the sinks its owner set, and the audit log (8.6) still records every capability call.
- The checker makes this one exception, for the functions of `std.log`.

## Alternatives considered

- **`uses Console`**, because the first sink prints. Every logging function would carry it, and the name becomes wrong once events go to files or the network instead.
- **A new `Log` capability.** Honest, but it spreads into nearly every signature for little gain, since the code can't choose the sink anyway. A program that wants to limit logging can do it in `kavach.toml` (levels, sampling, filters).
