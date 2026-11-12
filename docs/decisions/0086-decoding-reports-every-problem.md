# 0086. Decoding reports every problem, invariants included

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

Rath, a web framework written in Kavach, asked (2026-11-12) for decoding that reports every field error with its path, `require` failures included, not only the first. A web API answers a bad request once, so it should list everything wrong in it, as FastAPI and pydantic do.

Checking it found a gap: `json.decode<T>` never checked `T`'s `invariant`s, though spec 7.3 says an invariant is checked whenever a value of the type is made. A decoded `Item` could have `qty: 0` under `invariant qty > 0`, and text from outside is exactly where such values come from.

A maintainer chose (2026-11-12), from the options put to them, both answers below.

## Decision

- **One error lists every problem** (spec 2.12): decoding goes on after a problem and fails once with `JsonError.Invalid(problems: [JsonProblem])`, each `JsonProblem(path:, expected:)`, in the document's order. It replaces `Mismatch`.
- **A broken invariant in decoded input is a problem, not a panic.** The input is not a mistake in the program, and a panic would let a hostile request stop a handler on purpose. Its `path` is the record's, its `expected` ``invariant `qty > 0` ``. Invariants are checked once every value has the right shape.
- The type is named `JsonProblem`, not `Problem`: the types of `std.json` are in every program, which may well have a `Problem` of its own.

## Consequences

- A service returns all field errors in one answer, with paths a client can show beside each field.
- A value built by decoding keeps its type's invariants, as any other value does.
- Decoding a document that fits costs what it did: problems are collected only after the first, by a second pass that builds nothing. Types without invariants add no checks.

## Alternatives considered

- **The first problem by default, all of them on request** (`all_problems: true`): two behaviours to learn, and the useful one hidden.
- **A separate `json.check<T>`:** the text would be read twice when it is wrong, and decode would still say less.
- **A broken invariant as a panic,** as everywhere else a contract breaks: simple, but a hostile input could crash a handler.
