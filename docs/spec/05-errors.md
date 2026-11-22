# 5. Errors

Kavach has **no exceptions**. A function that can fail says so in its signature, and the caller must deal with it. Errors cannot be silently ignored.

## 5.1 Declaring errors

`! E` after the return type lists the errors a function can return.

```kavach
type AgeError = Empty | NotANumber(text: String) | TooLarge(value: Int)

fn parse_age(text: String) -> Int ! AgeError {
    if text.is_empty() {
        fail AgeError.Empty
    }
    const n = Int(text) catch _ => fail AgeError.NotANumber(text: text)
    if n > 150 {
        fail AgeError.TooLarge(value: n)
    }
    n
}
```

- `fail value` returns an error from the function.
- A function with no return value writes only `! E`: `fn save(user: User) ! DbError`.
- Error types are ordinary types. Union variants automatically get a readable message from their name and fields.

## 5.2 try: pass the error on

`try` returns the error to your caller if the call fails.

```kavach
fn load_profile(path: String) -> Profile ! ProfileError {
    const text = try read_file(path)
    const profile = try parse_profile(text)
    profile
}
```

`try` works when the callee's error is part of the caller's error type. For example, `ProfileError` could be `type ProfileError = FileError | ParseError`.

A built-in method that takes a function (`map`, `filter`, `sum`, `sort_by`, `Shared.update`, …) passes that function's error on: given a function that can fail with `E`, the call can fail with `E`, and the method stops at the first error. `update` then leaves the shared value as it was. A function written in Kavach does the same by saying so in its types (3.4).

```kavach
fn price(o: Order) -> Decimal ! PriceError { … }

const prices = try orders.map(price)              // fails with the first order's PriceError
const total = orders.sum(o => price(o) catch _ => 0.0)  // handled inside: can't fail
```

`try` or `catch` on something that can't fail is a warning, and does nothing: `Int("2") catch 0` is just `2`, since the compiler read that text already (2.1).

## 5.3 catch: handle the error here

```kavach
const age = parse_age(input) catch 0                    // use a default

const age = parse_age(input) catch e => {               // run code with the error
    print(`Bad input: {e}`)
    0
}
```

A handler may also leave instead of giving a value: `return` from the function, or `break` or `continue` in a loop.

A call's `catch` handles that call's own error, never one raised while its arguments are worked out: a `fail` or `try` in an argument leaves the function, as it would with no `catch` around the call.

```kavach
fn find(text: String) -> String ! AppError {
    lookup(try parse(text)) catch e => {     // `e` is a `LookupError`: a bad `text` leaves `find`
        return `nothing: {e}`
    }
}
```

```kavach
for line in lines {
    const n = Int(line) catch _ => {
        continue                                      // skip lines that aren't numbers
    }
    total += n
}
```

## 5.4 match: handle each case

```kavach
match parse_age(input) {
    ok(age) => print(`Age is {age}`)
    err(AgeError.Empty) => print("Please enter an age")
    err(AgeError.NotANumber(text)) => print(`'{text}' is not a number`)
    err(AgeError.TooLarge(value)) => print(`{value} is too large`)
}
```

## 5.5 The Error catch-all

For scripts and application code that just wants to stop on any error, `! Error` accepts every error type:

```kavach
fn main() ! Error {
    const config = try load_config("app.toml")
    const users = try fetch_users(config.api_url)
    print(`Loaded {users.len()} users`)
}
```

When `main` returns an error, the program prints it and exits with code 1.

When a script has no error type of its own to give, `Error(message)` makes an error that is only its message:

```kavach
fn check(age: Int) -> Int ! Error {
    if age < 0 {
        fail Error(`age can't be negative: {age}`)
    }
    age
}
```

Printing such an error prints its message.

An `Error` keeps the error it holds, and a `match` can ask which one it is: name the variant with its type, and end with `_` ([decision 0050](../decisions/0050-match-on-an-error-names-its-type.md)):

```kavach
const settings = load(path) catch e => match e {
    FsError.NotFound(path) => defaults(),
    JsonError.Syntax(line, column, message) => panic(`{path}:{line}: {message}`),
    _ => panic(`can't load {path}: {e}`),
}
```

- An arm matches when the error is of that type and that variant, whatever `context` was added to it (5.7).
- The type's name is required: `NotFound` alone is an error, since an `Error` belongs to no single type.
- The `match` must have a `_` arm (or a name), since an `Error` can hold any type, `Error("message")` too.

Use specific error types in libraries and `pub` APIs, so callers know exactly what can go wrong.

## 5.6 Panics

A **panic** is a bug, not an expected failure. It stops the current task. Panics happen on:

- a contract violation (chapter 7)
- an out-of-range index (`list[99]`)
- integer overflow
- integer division or remainder by zero
- `.unwrap()` on `none`
- running out of stack (very deep recursion, 3.8)
- asking for more memory than Kavach allows in one go, such as `"ab".repeat(n)` asking for more than 1 GB of text, or a list grown past its limit (2.2)
- an explicit `panic("message")`

**When the machine itself runs out of memory**, which Kavach can't see coming, the whole program ends: native code exits with code 101 and `panic: out of memory (asked for N bytes)`, or the system ends it first (on Linux, code 137). The interpreter, a tool for development, may end without that message. A server that must survive this caps what one request may ask for, as `ServeLimits` does (14.11).

Panics are not caught by normal code. An agent handler that panics is stopped, logged in the audit log, and the agent keeps serving other requests: the send that started it fails with `panicked: <message> at <file>:<line>` (8.8, [decision 0076](../decisions/0076-every-send-to-an-agent-can-fail.md)).

**Where a panic points.** A panic shows `at` the line where it happened. When that line is in the standard library's own code, the panic shows the line of the program's code that called into the library instead: the call that passed the wrong value, which is the line a person can change. A `require` of a library function points at the call the same way, as the caller has the bug (7.1). A crash report (14.30) still lists the library's functions that were running.

```kavach
use std.tensor

fn main() {
    const a = tensor.of_rows([[1.0, 2.0]])          // shape [1, 2]
    const b = tensor.of_rows([[1.0], [2.0], [3.0]]) // shape [3, 1]
    print(a @ b)    // panic: `@` needs [1, 2]'s last size (2) to match [3, 1]'s …
}                   //   at main.kav:6, not a line of std/tensor/ops.kav
```

Integer overflow is **always checked**, in debug and release builds alike. A silent wrap-around is a security bug. When wrapping is intended, say so: `a.wrapping_add(b)`. When overflow is possible and should be handled, use `a.checked_add(b)`, which returns `none` on overflow (also `checked_sub`, `checked_mul`, `checked_div`; `checked_div` returns `none` for a division by zero too).

## 5.7 Adding context

`context` adds a short explanation to an error that `try` passes on:

```kavach
fn load_settings() -> Settings ! Error {
    const text = try read_file("app.toml") context "loading settings"
    try parse_settings(text) context "reading app.toml"
}
```

The error's message becomes `loading settings: <original message>`. The original error value is kept, so callers can still `match` on it.

## 5.8 When a program stops

| What happened | Printed to standard error | Exit code |
|---|---|---|
| `main` returned an error | `error: <message>` | `1` |
| A panic in the main task | `panic: <message>`, then `  at <file>:<line>` | `101` |
| Ctrl+C (`SIGINT`) | nothing | `130` |
| The terminal or console closed (`SIGHUP`) | nothing | `129` |
| `SIGTERM` (how Docker and systemd stop a program; on Windows, logging off or shutting down) | nothing | `143` |

Ctrl+C, a closed terminal and `SIGTERM` end the program the way `cli.exit` does: what it printed is flushed, and logs and traces are sent before it ends ([decision 0062](../decisions/0062-crashes-restarts-and-clocks.md)). A second Ctrl+C ends it at once.

**Finishing before a stop.** A program with work it must not cut off (requests being answered, a file half written, a queue to empty) takes a hold with `cli.on_stop(grace)` (14.6). While a hold is kept, the first stop signal doesn't end the program: `hold.asked()` turns `true`, and the program finishes what it is doing. It ends, with the signal's exit code, once every hold is let go of with `hold.done()`, or when the longest `grace` has passed, or when `main` returns; a second signal ends it at once. `http.serve` takes a hold of its own, so a server turns new connections away and finishes the requests under way before the program ends (14.11, [decision 0088](../decisions/0088-finishing-before-a-stop.md)).

```kavach
use std.cli
use std.time

fn main() uses Clock {
    const hold = cli.on_stop(grace: 30.seconds)
    while not hold.asked() {
        work_a_little()
    }
    save_progress()         // Docker's SIGTERM: this still runs
    hold.done()
}
```

In debug builds, an error also records every `try` it passed through. When `main` returns the error, this **error trace** is printed below the message, so you can see the path the error took without a debugger.

These messages never go to standard output, so a program's real output stays clean.

## 5.9 Watching for NaN

A float operation with no answer gives `NaN`, as IEEE 754 says: `0.0 / 0.0`, `inf - inf`, `math.sqrt(-1.0)`. The program goes on, and the `NaN` spreads through every sum it touches, so a model's loss turns into `NaN` far from the bug. The **NaN watch** stops the program at the first operation that makes one, and names it:

```toml
[run]
watch_nan = true
```

```kavach
fn mean(xs: [Float]) -> Float {
    let total = 0.0
    for x in xs {
        total += x
    }
    total / Float(xs.len())      // 0.0 / 0.0 for an empty list
}

fn main() {
    print(mean([]))
}
```

```
panic: NaN watch: 0.0 / 0.0 gives NaN
  at mean.kav:6
```

- **Watched:** the operators `+`, `-`, `*`, `/`, `%` and `**` on `Float` and `Float32`, `+=` and the others too, and `std.math`'s functions of floats (14.5), as the program's code writes them. When one gives `NaN`, the program panics (5.6) with the operation and its operands: `NaN watch: inf - inf gives NaN`, `NaN watch: math.sqrt(-1.0) gives NaN`. `math.pow(x, y)` shows as `x ** y`. A `NaN` that comes from outside, from a file or `Float.parse`, stops the program at the first watched operation it reaches.
- **Not watched:** the standard library's own code, which makes `NaN` on purpose (a missing value in a table, 14.34); methods of lists, such as `sum`; and a `map` that runs on the GPU (15.2).
- **Turning it on:** `watch_nan = true` under `[run]` in `kavach.toml` (10.11), or `--watch-nan` for one run or build: `kavach run --watch-nan train.kav`. It works the same in the interpreter, `kavach run --native` and `kavach build`, which names it in the program's build line (10.5). `kavach test` doesn't watch: a test may make `NaN` on purpose, to check what a function does with it.
- **Its cost:** off, as it is by default, nothing: the checks are compiled only into a program that asks for them. On, each watched operation goes through the runtime and is checked, so a loop of float arithmetic runs about seven times slower; it is for finding a bug, not for production.

## Open questions

- Where a panic in the standard library points (5.6, chosen 2026-11-22, for a maintainer's review): at the program's innermost call into the library that is running, as Rust's `#[track_caller]` points at the caller. A package's (10.12) panics still point inside the package, and a library function called through a function value points inside the library in native code. Native code pays for it only on calls of library functions that can panic: two quick calls into the runtime each.
- The NaN watch (5.9, chosen 2026-11-23, for a maintainer's review): a setting read when the program is compiled, so it costs nothing when off, rather than a check while it runs or the processor's floating-point traps (fast when on, but each system reports them its own way). It watches the program's own code and stops at the first `NaN`, even one that came from outside; not yet the methods of lists (`sum`, `mean`) or the GPU.

Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
