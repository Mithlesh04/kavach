# 0094. A function that returns a secret says so

- **Status:** Accepted
- **Date:** 2026-11-12

## Context

A secret (spec 8.7) was tracked inside a function body, but not through a call's result. A helper that read a secret gave plain text to its caller:

```kavach
fn read_db() -> String ! Error uses Env {
    try env("DB")
}

print("db is {try read_db()}")      // printed the password, with no `reveal`
```

Rath, a web framework written in Kavach, found this (2026-11-12) while writing a settings loader. Every `get_api_key()` helper had the same hole. A maintainer chose (2026-11-12), from the options put to them, the rule below.

## Decision

- A function whose result is a secret writes `-> secret T`, as a parameter writes `secret card: String`.
- Returning a secret, with `return` or as the last line, from a function whose result isn't `secret` is a compile error.
- A call to a `-> secret T` function gives a secret, which goes where any secret may go (8.7).
- A lambda can't return a secret: a function type can't say so, so a named function does it.
- An interface method and the methods that implement it agree on `secret` in the result.

## Consequences

- No helper can turn a secret into plain text without `reveal`, which `kavach audit` lists.
- The rule shows in signatures, in review and in `kavach outline`.
- Generic functions need nothing new: a secret can't reach a parameter that isn't `secret`.

## Alternatives considered

- **Secrecy inferred through results:** no new syntax, but invisible in the signature, and a package's callers couldn't see it.
