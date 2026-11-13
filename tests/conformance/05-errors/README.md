# 5. Errors

Conformance tests for spec chapter [5. Errors](../../../docs/spec/05-errors.md). Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 5 (introduction) | [errors-cannot-be-ignored.kav](errors-cannot-be-ignored.kav) | The result of a call that can fail must be handled with try, catch or match. | compile error |
| 5.1 | [fail-and-catch-default.kav](fail-and-catch-default.kav) | `fail` returns an error; `catch value` uses a default instead. | runs |
| 5.1 | [errors-can-be-any-type.kav](errors-can-be-any-type.kav) | A function can fail with an `Int` or a `Float`; `try`, `catch` and `main` treat it as any other error. | runs |
| 5.2 | [try-needs-a-matching-error-type.kav](try-needs-a-matching-error-type.kav) | `try` only works when the callee's error is part of the caller's error type. | compile error |
| 5.2 | [try-passes-errors-on.kav](try-passes-errors-on.kav) | `try` returns the error to the caller when a call fails. | runs |
| 5.3 | [catch-with-handler.kav](catch-with-handler.kav) | `catch e => { … }` runs code with the error and gives a value. | runs |
| 5.3 | [catch-handler-can-leave.kav](catch-handler-can-leave.kav) | A `catch` handler may leave instead of giving a value: `return` from the function, or `continue` and `break` in a loop. | runs |
| 5.3 | [a-calls-catch-handles-only-its-own-error.kav](a-calls-catch-handles-only-its-own-error.kav) | A call's `catch` handles only the call's own error: a `fail` or `try` in its arguments leaves the function. | runs |
| 5.5 | [main-error-exits-with-1.kav](main-error-exits-with-1.kav) | When main returns an error, the program exits with code 1. | exit 1 |
| 5.5 | [error-with-a-message.kav](error-with-a-message.kav) | `Error(message)` makes an error that is only its message; `main` returning it exits with code 1. | runs |
| 5.5 | [match-on-an-error-names-its-type.kav](match-on-an-error-names-its-type.kav) | A `match` on an `Error` names variants with their type and ends with `_`; an arm matches the error's type and variant, through any `context`. | runs |
| 5.5 | [match-on-an-error-needs-the-type-name.kav](match-on-an-error-needs-the-type-name.kav) | A variant pattern on an `Error` names its type: `NotFound(path)` alone is an error, since an `Error` belongs to no single type. | compile error |
| 5.5 | [match-on-an-error-needs-a-catch-all.kav](match-on-an-error-needs-a-catch-all.kav) | A `match` on an `Error` must have a `_` arm, since an `Error` can hold any type. | compile error |
| 5.6 | [checked-add.kav](checked-add.kav) | checked_add returns none on overflow. | runs |
| 5.6 | [division-by-zero-panics.kav](division-by-zero-panics.kav) | Integer division by zero panics. | panic |
| 5.6 | [explicit-panic.kav](explicit-panic.kav) | panic("message") stops the task with that message. | panic |
| 5.6 | [index-out-of-range-panics.kav](index-out-of-range-panics.kav) | An out-of-range index panics. | panic |
| 5.6 | [loop-write-panics-at-its-index.kav](loop-write-panics-at-its-index.kav) | A write past a list's end panics when the loop reaches it, after the writes and output before it. | panic |
| 5.6 | [integer-overflow-panics.kav](integer-overflow-panics.kav) | Integer overflow is always checked and panics. | panic |
| 5.6 | [loop-overflow-panics-for-a-large-multiplier.kav](loop-overflow-panics-for-a-large-multiplier.kav) | Overflow in a loop panics for the values that overflow it, even where other values run without a check. | panic |
| 5.6 | [narrow-division-overflow-panics.kav](narrow-division-overflow-panics.kav) | Dividing a sized integer's smallest value by a constant -1 panics with an overflow, as any overflow does; `%` by -1 gives 0. | panic |
| 5.6 | [wrapping-add.kav](wrapping-add.kav) | wrapping_add wraps around instead of panicking. | runs |
| 5.6 | [panic-with-a-computed-message.kav](panic-with-a-computed-message.kav) | `panic(message)` may compute its message while the program runs. | panics |
| 5.7 | [error-context.kav](error-context.kav) | `context` adds text in front of a passed-on error's message. | runs |
| 5.8 | [panic-exits-with-101.kav](panic-exits-with-101.kav) | A panic in the main task exits with code 101. | panic |
| 5.2 | [built-in-methods-pass-a-functions-error-on.kav](built-in-methods-pass-a-functions-error-on.kav) | A built-in method given a function that can fail can fail with its error, stopping at the first; one handled inside can't fail. | runs |
| 5.2 | [try-deep-inside-an-argument.kav](try-deep-inside-an-argument.kav) | A call that fails keeps its error when an argument holds a `try` deeper inside: a function, a function value or a standard library call. | runs |
