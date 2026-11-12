# 8.1 Capabilities

A capability is permission to affect the outside world. Built-in capabilities:

| Capability | Allows |
|---|---|
| `Console` | Printing and reading the terminal |
| `Fs` | Reading and writing files |
| `Net` | Network access |
| `Env` | Environment variables, process arguments |
| `Clock` | Waiting: `sleep`, intervals and timers (14.4). Reading the time needs none |
| `Random` | Random numbers |
| `Ai` | Calling AI models (`ask<T>`) |
| `Clipboard` | Reading and writing the clipboard (14.17): what a person copied is private |
| `Serial` | Serial ports (14.18): boards, sensors, modems and other devices on a cable |
| `Approvals` | Answering agents' approval requests in code (14.16). An agent's `can` may never list it (8.3) |
| `Foreign` | Calling Python or JavaScript code, which Kavach cannot check (10.7) |

Functions list the capabilities they use with `uses`:

```kavach
pub fn fetch_rates(url: String) -> Rates ! HttpError uses Net {
    const body = try http.get(url)
    try json.decode<Rates>(body)
}
```

Rules:

1. A function **without** `uses` is pure. It cannot affect the outside world at all.
2. A function may only call functions whose capabilities it also has.
3. `pub` functions **must** write their `uses` clause. Private functions may leave it out; the compiler infers it.
4. `main` has every built-in capability.
5. `debug()` needs no capability (it is removed from release builds).
6. Logging with `std.log` needs no capability: the program's owner chooses where events go, not the code that writes them ([14.12](../14-standard-library/12-log.md), [decision 0023](../../decisions/0023-logging-needs-no-capability.md)).

A function value carries the rights of the code that made it: naming a function as a value, or writing a lambda, needs what that function uses, as calling it would. Calling a function value needs nothing more, so a library can call the functions it is given (a web framework its handlers) without being granted what they use (10.12). A function type can limit what its functions may use with `uses` (3.4, [decision 0085](../../decisions/0085-function-types-with-errors-and-uses.md)).

```kavach
fn twice(f: fn(Int) -> Int, n: Int) -> Int {     // pure: it only calls what it is given
    f(f(n))
}

pub fn shout(n: Int) -> Int uses Console {          // pays for the lambda's `print`
    const loud = x => {
        print(x)
        x + 1
    }
    twice(loud, n)
}
```

Reading a signature tells you what code can do:

```kavach
fn total(orders: [Order]) -> Decimal                    // cannot touch anything
fn save(order: Order) ! DbError uses Fs                 // can touch files, nothing else
fn sync(order: Order) ! SyncError uses Net, Payments    // network and payments
```

### Narrower file access

`Fs` can be granted in part:

| Written | Allows |
|---|---|
| `uses Fs` | Every file operation |
| `uses Fs.read` | Only reading: `read_text`, `read_bytes`, `lines`, `chunks`, `exists`, `list`, `info` |
| `uses Fs.write` | Only changing: `write_text`, `write_bytes`, `append_text`, `remove`, `create_dir`, `set_read_only`, `set_permissions`, `rename`, `copy`, `remove_all` |
| `uses Fs("./data")`, `uses Fs.read("./data")` | The same, only inside the folder `./data` |

`Fs.read` and `Fs.write` are checked by the compiler, like any capability. A folder is checked when the program runs, since paths are values: while a function with `uses Fs("./data")` runs, every file operation in it, and in everything it calls, must name a path inside `./data`, or it fails with `Denied(path)` (`fs.exists` gives `false`). `..` can't leave the folder, and neither can a symbolic link that points outside it. Folders nest: inside two of them, a path must be in both. A task keeps the folders of the function that spawned it.

```kavach
fn load_report(name: String) -> String ! FsError uses Fs.read("./reports") {
    try fs.read_text("./reports/{name}.txt")
}

const report = load_report("../secrets") catch e => ""   // Denied: outside ./reports
```

The folder is relative to the folder the program runs in, as paths are (14.3).
