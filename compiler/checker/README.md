# compiler/checker (`kavach-checker`)

Checks types and the rules that need them (spec chapters 2–7), on one parsed and resolved file. Every message says how to fix the problem:

| Rule | Example message |
|---|---|
| Types match; no implicit number conversions (2.1) | there are no implicit number conversions: expected `Float`, found `Int`; convert with `Float(…)` |
| No null (2.7) | `Int` can't hold `none`: write `Int?` for a value that may be missing |
| Records use named arguments and every field (2.4) | missing field `email` for `User` |
| Interfaces are explicit (2.8) | `User` does not implement `Describe`: write `type User: Describe` or `extend User: Describe` |
| Ambiguous variants need their union (2.6) | `Open` is a variant of several types: write `Door.Open` or `Shop.Open` |
| Secrets stay secret (2.11) | `card_number` is secret, so it can't go into `print` |
| Functions without `->` return nothing (3.1) | `double` has no `->`, so it returns nothing, but its last line is a value: write `-> Int` |
| Results are used (3.7) | the result of `add` is unused: use it, or write `const _ = …` to throw it away |
| Conditions are `Bool` (4.1) | conditions must be `Bool`, not `Int`: there is no truthiness |
| `match` covers every case (4.3) | this `match` does not cover `Refunded` |
| Errors are handled (5) | `check_age` can fail: handle the error with `try`, `catch` or `match` |
| `try` fits the caller's errors (5.2) | `try` passes the error on, but `double_age` has no `!` in its signature |
| Arena values don't escape (6.3) | this value was made inside the `arena` and can't be stored outside it |
| `unsafe` needs opt-in (6.4) | this package doesn't allow `unsafe`: set `unsafe = true` in kavach.toml |
| Contracts are pure (7.5) | a contract can't call `noisy_check`: it uses Console |

Not checked yet: capability inference and `uses` rules, and the expressions inside an agent's `limit` and `approve` rules (spec chapter 8). These come with Phase 2.

## How it works

Hindley–Milner-style inference with unification. A number literal starts as an unknown type that may only become a number (`0.1` only a `Float` or `Decimal`), so context decides its type (spec 1.7). A call to a function with `! E` has type `Result(T, E)`, which only `try`, `catch`, `match` and `example` accept. Anywhere else, it is reported as an unhandled error.

## Files

| File | Job |
|---|---|
| `src/lib.rs` | Public API: `check`, `Options`, `Checked` |
| `src/ty.rs`, `src/infer.rs` | Types; type variables, unification, substitution |
| `src/checker.rs` | State, type display, coercion (`T` fits `T?`), mismatch messages |
| `src/collect.rs` | Written types → checker types; signatures of functions, types and methods |
| `src/drops.rs` | `Drop` (spec 6.1): which types implement it, which values hold one, and that making one needs what its `drop` uses |
| `src/items.rs` | Declarations, function bodies, blocks, statements, unused values |
| `src/exprs.rs` | Expressions: names, operators, lambdas |
| `src/failures.rs` | `try`, `fail`, `return`, error compatibility, `unsafe` |
| `src/calls.rs` | Calls, record and variant construction, arguments |
| `src/methods.rs` | Method calls: a type's own, built-ins, `.timeout(d)`, and fields holding functions (spec 3.4) |
| `src/c_functions.rs` | C functions (spec 10.7): the types their signatures may use, and the Kavach functions C calls back |
| `src/fields.rs` | Fields, indexing, `with`, and arena escapes |
| `src/patterns.rs` | Patterns, `if`/`match` branches, match coverage |
| `src/builtins.rs` | Prelude functions |
| `src/builtin_methods.rs` | Methods of built-in types, and `x.timeout(d)` |
| `src/stdlib.rs` | The standard library: `Set`, `use std.<module>` and its functions (spec 14) |
| `src/more_methods.rs` | More methods of built-in types: text, lists, maps, numbers (spec 14.2) |
| `src/order.rs` | Which values have an order (spec 1.8), and the waits refused inside `Shared.update` (spec 6.2) |
| `src/method_names.rs` | The name of every built-in method, for `builtin_methods` (code completion); a test keeps it in step with the method tables |
| `src/processes.rs` | `spawn process` (spec 9.8): a call of a named, non-generic function whose arguments, result and error JSON can carry |
| `src/json.rs` | `Json` (spec 2.12): literals that become one where it is expected, and which types can be encoded |
| `src/capabilities.rs` | Built-in capabilities: inferring and checking `uses` (spec 8.1) |
| `src/gpu.rs` | `gpu fn` (spec 15.1): its signature, its number literals, no recursion, and which `map`s may run on the GPU |
| `src/gpu_walk.rs` | The walk over a `gpu fn`'s body: what it may use, with a message naming anything else |
| `src/always_panics.rs` | Arithmetic that always panics (spec 1.8): division by a known `0`, `+ - *` of known values that can't fit, an index past a known list's end; `1 / 2` warns |
| `src/deprecated.rs` | Calls to a deprecated function warn with its message (spec 12.6) |
| `src/never_runs.rs` | Code that never runs (spec 4.7): `match` arms that can't match; `never_runs/chains.rs`, an `else if` an earlier condition covers; `never_runs/loops.rs`, a `while` that never ends, and a `for` over `0..=xs.len()` reading `xs[i]` (4.4) |
| `src/gpu_fit.rs` | Whether a plain `fn` would fit the GPU's rules, for the message when a `gpu fn` calls one |
| `src/secrets.rs` | Secrets (spec 2.11, 8.7): where a secret may go, and a secret returned without `-> secret` |
| `src/std_crypto.rs` | The signatures of `std.crypto` (spec 14.19) |
| `src/std_base64.rs` | The signatures of `std.base64` (spec 14.20) |
| `src/scoped.rs` | Blocks given to a call (spec 4.8): which calls take one, their arguments, and the error for one without its block |
| `src/std_devices.rs` | The signatures of serial ports' and the clipboard's built-in functions (spec 14.18, 14.17) |
| `src/std_writers.rs` | The signatures of `FileWriter`'s built-in functions, by the writer's number (spec 14.3) |
| `src/std_log.rs` | The signatures of `std.log`'s built-in functions: metrics, the trace's id and `traceparent` (spec 14.12) |
| `src/std_compress.rs` | The signatures of `std.compress`'s built-in functions (spec 14.21) |
| `src/std_gpu.rs` | The signatures of `std.gpu`'s built-in functions, screens and frames by number (spec 14.24) |
| `src/std_process.rs` | The signatures of `std.process`'s built-in functions, by the child's number (spec 14.23) |
| `src/std_window.rs` | The signatures of `std.window`'s built-in functions, by the window's number (spec 14.22) |
| `src/cli_parse.rs` | `cli.parse<T>()` (spec 14.6): a record of options, or a union of commands |
| `tests/types.rs` | Valid programs pass; each mistake gets its message on the right line |
| `tests/from_other_languages.rs` | Habits from other languages (`===`, `null`, inheritance, `2nd_place`, …) get one clear error that says what Kavach does instead |
| `tests/gpu_messages.rs` | A `gpu fn` calling a plain `fn` says whether it fits the GPU's rules, or what keeps it off |
| `tests/spec_sync.rs` | Every conformance test and example: no false errors, and every compile-error test caught on its marked line by some stage |
