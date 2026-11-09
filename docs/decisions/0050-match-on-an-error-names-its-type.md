# 0050. A `match` on an `Error` names the error's type

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

`! Error` accepts every error type (spec 5.5), so scripts and applications can pass on whatever goes wrong with `try`. The code that finally handles such an error often needs to know what it was: a missing file is shown differently from a broken JSON file. Python asks with `except FileNotFoundError`, Go with `errors.As`. Kavach had no way: an `Error` could only be printed.

The value is already there. `try` keeps the original error inside the `Error`, and `context` (5.7) wraps it without losing it, in the interpreter and in native code.

## Decision

**A `match` on an `Error` may name variants of any error type, written with the type's name, and must end with `_`.** Mithlesh chose this on 2026-11-03 (option a of three).

```kavach
const settings = load(path) catch e => match e {
    FsError.NotFound(path) => defaults(),
    JsonError.Syntax(line, column, message) => panic("{path}:{line}: {message}"),
    _ => panic("can't load {path}: {e}"),
}
```

- The type's name is required (`FsError.NotFound`, not `NotFound`): an `Error` has no single type that a bare name could belong to.
- An arm matches when the error is of that type and that variant, whatever `context` was added on the way (5.7).
- `_` (or a name) is required, because an `Error` can hold any type, `Error("message")` too.

## Consequences

- Code that passes errors on as `! Error` can still handle the ones it knows, with no wrapper types.
- A pattern on an `Error` checks the error's type as well as its variant, so `FsError.Denied` never matches an `InboxError.Denied`.
- The checker can't tell whether an `Error` can ever hold a `FsError`: an arm that never matches is not an error.

## Alternatives considered

- **(b) A method that asks:** `e.is<FsError>()` and `e.as<FsError>()`, as Go does. It takes two steps where one `match` does, and a new kind of method.
- **(c) Nothing:** code that cares writes its own error union. That's the advice for libraries (5.5), but it's heavy for an application's last line of handling.
