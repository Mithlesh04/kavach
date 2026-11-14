# 0108. No user-defined decorators

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

Many languages let code wrap a function in another: Python's `@decorator`, TypeScript's `@Component`, Java's annotations read through reflection, Rust's attribute macros. A maintainer asked (2026-11-15) whether Kavach should have them.

Kavach promises that a function's signature says everything it does: what it takes and returns, how it can fail (`!`, chapter 5), what it can touch (`uses`, 8.1) and what it promises (chapter 7). `kavach outline` and `kavach context` (spec 13) show only signatures, and AI tools and readers skip the bodies because of that promise.

## Decision

Kavach has no user-defined decorators: nothing written above a function can change what it does.

What decorators do in other languages, Kavach says in the signature or in plain code:

| Decorator use elsewhere | In Kavach |
|---|---|
| Checking arguments and results (`@validate`) | `require`, `ensure`, `invariant` (chapter 7) |
| Permissions, rate limits | `uses` (8.1); an agent's `can`, `limit` and `approve` (8.3) |
| Tests and examples (`@test`) | `example` clauses and `test` blocks (chapter 11) |
| Tracing and timing | `log.span("name") { … }` (4.8, 14.12) |
| Generated equality, printing, JSON (`@dataclass`, `derive`) | Every record has them already (1.8, 2.12) |
| Registering a route (`@app.route`) | A call that adds it to a router |

When a need keeps coming up, it becomes a clause the compiler knows and checks, written in the signature beside `uses`, never an open-ended wrapper. Caching a pure function's results is the likely first (language request 10 (9)), and retrying a call that failed the next.

## Consequences

- The outline stays the truth: no wrapper adds a capability, an error, a retry or a cache that the signature doesn't show.
- Capabilities, errors and agent limits are checked on what the code says, with nothing in between.
- AI tools write and read one way of doing each thing.
- A program can't build its own wrappers; a library that wants one offers a function that takes a function or a block.

## Alternatives considered

- **Decorators as in Python:** any function can wrap any other. Rejected: a wrapper can do I/O, swallow errors or change the result type unseen by the signature.
- **Decorators the compiler checks:** a wrapper's own `uses` and errors added to the function it wraps. Rejected for now: the signature would no longer be what is written, and readers would have to look up each wrapper.
- **Attribute macros, as in Rust:** code that writes code at compile time. Rejected: generated code no one wrote is hard to read and to check, and the language already gives records what `derive` gives most often.
