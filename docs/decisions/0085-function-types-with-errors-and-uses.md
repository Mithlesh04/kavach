# 0085. Function types with errors and `uses`, and describing a function

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Rath, a web framework written in Kavach in a repository of its own, asked (2026-11-12) to take handlers such as `fn create(o: NewOrder) -> Order ! ApiError uses Db` in `.post(path, create)`, to decode a request into the handler's input, and to write an OpenAPI description from the handler's signature. Two gaps stood in the way:

- A function type could not name errors or capabilities: `f: fn(P) -> R ! E` did not parse. Worse, a function that can fail was accepted where a `fn(P) -> R` was expected. The interpreter then let its error escape from code that declares none, and native code dropped the error and went on with a made-up value.
- A `try` inside a lambda passed its error to the function the lambda was written in, a rule the spec never stated. It is wrong once the lambda leaves that function: whoever calls it later gets an error it never declared.

Capabilities had no such hole: the code that makes a function value, by naming a function or writing a lambda, must have what that function uses (spec 8.1), so a function value carries the rights of the code that made it.

A maintainer chose (2026-11-12), from the options put to them: errors and `uses` in function types, with the error type generic; and handlers that take one record, plus a way to describe a function.

## Decision

- **A function type says what its functions can fail with and may use** (spec 3.4): `fn(NewOrder) -> Order ! ApiError uses Db`. Several capabilities go in parentheses, `uses (Net, Db)`, since a `,` after a type in a parameter list or a tuple would otherwise be ambiguous.
- **Errors are part of a function value's type.** A function that can fail fits only a function type that says so, with an error its own fits. One that can't fail fits any. The error may be a type parameter, `fn route<P, R, E>(f: fn(P) -> R ! E)`, so a framework keeps the handler's own error type and a `match` on it stays exhaustive. Calling a function value that can fail is a call that can fail (spec 5): `try`, `catch` or `match`.
- **A lambda's `try` and `fail` are the lambda's** (spec 3.3): they end the lambda's call with the error, not the function around it.
- **Built-in methods that take a function pass its error on** (spec 5.2), as Swift's `rethrows` does: `try orders.map(o => price(o))`. The method stops at the first error. `Shared.update` leaves the value as it was.
- **`uses` in a function type is a limit.** A function value may use only what its type lists. A type without `uses` sets no limit, as a private function without `uses` gets what its body needs (8.1). The code that makes the value still pays for what it uses, so a package granted only `Net` (decision 0084) can call an application's handler that uses a database. A capability type parameter (`uses C`) would add nothing to that, so there is none.
- **`json.describe(f)`** (spec 2.12) gives a constant `FnSchema` for a function named in the program: its name, doc comment, parameters with their names and JSON Schemas, the schemas of its result and its error, what it `uses`, and its `example` clauses as text. A framework decodes a request into the one record a handler takes (`json.decode<P>`) and writes OpenAPI from `json.describe` and `json.schema<P>()`.

## Consequences

- A handler that fails, a callback that fails, and a function that fails passed through generic code are all checked. The error can't be lost, and the interpreter and native code agree.
- `xs.map(x => try check(x))` no longer compiles. It becomes `try xs.map(x => check(x))`: the call that can fail is the one marked.
- In native code, a lambda that can fail is a closure, not inlined into the loop, so it is slower than one that can't. Making it as fast is left for later.

## Alternatives considered

- **Errors only, no `uses` in function types:** simpler, but a framework could not say what a handler may touch.
- **The caller pays for a function type's `uses`:** a framework would then need every capability of every application it serves granted to it, which is what decision 0084's `can` exists to prevent.
- **Several named parameters and a dynamic call** (`call_json(f, args)`), as FastAPI does: the call would be checked only when the program runs.
- **Compile-time code (macros):** the most powerful, and a large feature of its own; not needed for this.
