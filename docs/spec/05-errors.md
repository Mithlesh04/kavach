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

`try` or `catch` on something that can't fail is a warning, and does nothing: `Int("2") catch 0` is just `2`, since the compiler read that text already (2.1).

## 5.3 catch: handle the error here

```kavach
const age = parse_age(input) catch 0                    // use a default

const age = parse_age(input) catch e => {               // run code with the error
    print("Bad input: {e}")
    0
}
```

A handler may also leave instead of giving a value: `return` from the function, or `break` or `continue` in a loop.

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
    ok(age) => print("Age is {age}")
    err(AgeError.Empty) => print("Please enter an age")
    err(AgeError.NotANumber(text)) => print("'{text}' is not a number")
    err(AgeError.TooLarge(value)) => print("{value} is too large")
}
```

## 5.5 The Error catch-all

For scripts and application code that just wants to stop on any error, `! Error` accepts every error type:

```kavach
fn main() ! Error {
    const config = try load_config("app.toml")
    const users = try fetch_users(config.api_url)
    print("Loaded {users.len()} users")
}
```

When `main` returns an error, the program prints it and exits with code 1.

When a script has no error type of its own to give, `Error(message)` makes an error that is only its message:

```kavach
fn check(age: Int) -> Int ! Error {
    if age < 0 {
        fail Error("age can't be negative: {age}")
    }
    age
}
```

Printing such an error prints its message.

An `Error` keeps the error it holds, and a `match` can ask which one it is: name the variant with its type, and end with `_` ([decision 0050](../decisions/0050-match-on-an-error-names-its-type.md)):

```kavach
const settings = load(path) catch e => match e {
    FsError.NotFound(path) => defaults(),
    JsonError.Syntax(line, column, message) => panic("{path}:{line}: {message}"),
    _ => panic("can't load {path}: {e}"),
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
- running out of memory, such as `"ab".repeat(n)` asking for more than 1 GB of text
- an explicit `panic("message")`

Panics are not caught by normal code. An agent handler that panics is stopped, logged in the audit log, and the agent keeps serving other requests: the send that started it fails with `panicked: <message> at <file>:<line>` (8.8, [decision 0076](../decisions/0076-every-send-to-an-agent-can-fail.md)).

Integer overflow is **always checked**, in debug and release builds alike. A silent wrap-around is a security bug. When wrapping is intended, say so: `a.wrapping_add(b)`. When overflow is possible and should be handled, use `a.checked_add(b)`, which returns `none` on overflow (also `checked_sub`, `checked_mul`, `checked_div`).

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

In debug builds, an error also records every `try` it passed through. When `main` returns the error, this **error trace** is printed below the message, so you can see the path the error took without a debugger.

These messages never go to standard output, so a program's real output stays clean.

## Open questions

None right now. Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
