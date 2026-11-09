# 10. Modules and Packages

## 10.1 Modules

- **One file is one module.** `orders.kav` is the module `orders`.
- **A folder groups modules.** `src/billing/invoice.kav` is the module `billing.invoice`.
- Everything is **private to its module** unless marked `pub`.
- A record with a private field can be **created only in its own module**. Other modules call a `pub fn` there that makes one, so the module keeps control of what its private fields hold.

```kavach
pub type Invoice {
    pub id: Int
    pub total: Decimal
    internal_note: String          // private field
}

pub fn create(order: Order) -> Invoice { … }
fn round_total(x: Decimal) -> Decimal { … }    // private
```

## 10.2 use

```kavach
use orders                          // then write orders.total(...)
use billing.invoice                 // then write invoice.create(...)
use billing.invoice.{Invoice, create}   // bring names in directly
use std.json                        // standard library
use stripe                          // a dependency, by package name
use std.collections.Set as HashSet  // rename
```

Resolution order: local modules (from `src/`), then `std`, then dependencies. A local module with the same name as a dependency is a compile error. Rename one of them.

The **prelude** is always available without `use`: built-in types, `print`, `debug`, `ask`, `panic`, `env`, `Shared`, `Channel`.

## 10.3 Project layout

```
my-app/
├── kavach.toml          project settings and dependencies
├── kavach.lock          exact dependency versions (generated, commit it)
├── src/
│   ├── main.kav         entry point (for applications)
│   ├── orders.kav
│   ├── orders.test.kav  hand-written tests of `orders` (few; see 11.8)
│   └── billing/
│       ├── invoice.kav
│       └── invoice.test.kav
└── tests/               stories across many modules
    └── found/           failing inputs found by `kavach test` (generated, commit it)
```

A library has `src/lib.kav` instead of `src/main.kav`.

A file named `name.test.kav` is the tests of `name.kav` beside it, not a module of its own (11.8). So a glance at a folder shows which modules have hand-written tests.

Build output and caches go to `.kavach/`, which is never committed: `kavach build` writes the executable to `.kavach/bin/<name>` (`<name>.exe` on Windows), where `<name>` is the package name.

## 10.4 kavach.toml

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2026"
unsafe = false            # must be true to use `unsafe` blocks (chapter 6)

[dependencies]
stripe = "1.2"
```

Other sections are described where they are used: AI models (`[ai]`) and agents' folder (`[agents]`) in chapter 8, release builds (`[build.release]`) in chapter 7, tests (`[test]`) in chapter 11, and limits, workspaces and architecture (`[limits]`, `[workspace]`, `[architecture]`) in chapter 12. Every key, with its values and default, is listed in 10.11.

## 10.5 Command line

| Command | Does |
|---|---|
| `kavach new my-app` | Create a project |
| `kavach run` | Compile quickly and run (a project, or one file: 10.9) |
| `kavach run --dry-run` | Run without executing capability calls |
| `kavach build` | Compile to an executable file (a project, or one file: 10.9) |
| `kavach build --release` | Optimized build |
| `kavach build --lib` | Compile a library into a shared library with a C header, for other languages (10.10) |
| `kavach test` | Run generated and hand-written tests (chapter 11) |
| `kavach fmt` | Format all code |
| `kavach add stripe` | Add a dependency |
| `kavach audit` | List unsafe code, revealed secrets and capability use across all dependencies |
| `kavach approvals` | List the requests a project's agents wait on: number, call, role, when asked, until when (8.3) |
| `kavach approve <id> --as <name>` / `kavach deny <id> --as <name> [--reason <text>]` | Answer one; the program waiting sees it within a second, or the next start finds it (8.3) |
| `kavach proto` | Write the `.proto` file for a program's types, so other languages' gRPC tools can talk to it (14.14) |
| `kavach fix` | Apply safe fixes: compiler suggestions, deprecations (12.6), edition upgrades (10.8) |
| `kavach api check` | Find breaking changes in `pub` signatures before a release (12.5) |
| `kavach repl` / `kavach eval` | Try code interactively or evaluate one expression (10.9) |
| `kavach --version` | Print the compiler's version and the commit it was built from: `kavach 0.1.0 (commit 8f8ae051)` |
| `kavach version <program>` | Print what built an executable: its package's name and version, and the compiler's version and commit (below) |
| `kavach lsp` | Language server for editors, on stdin and stdout: errors as you type, with the same messages as `kavach check`; the type of what is under the cursor (hover); where a name is declared (go to definition); and the names in scope, or after a `.` the fields and methods of the value (completion) |

Not built yet: `kavach add`, `fix`, `api check`, `repl` and `eval` (Phase 3 tools, [roadmap](../vision/roadmap.md)), and `build --release`: `kavach build` already optimizes. Until then they print the command list.

`kavach new my-app` creates the folder `my-app` with the layout of 10.3: a `kavach.toml`, a `src/main.kav` that prints a greeting, and a `.gitignore` for `.kavach/`. The name must be lowercase `kebab-case`.

Inside a project (a folder with `kavach.toml`, or any folder below it), `run`, `build`, `test` and `check` without a file work on the project: its `src/main.kav`. Arguments after `--` go to the program: `kavach run -- --verbose`.

`kv` is a short alias for `kavach`.

**A program's version.** Every executable `kavach build` writes carries, as text inside it, its package's `name` and `version` (10.4) and the version and commit of the compiler that built it. `kavach version` prints them, so a program found on a server can be traced to its source:

```
$ kavach version .kavach/bin/my-app
my-app 0.1.0
built by kavach 0.1.0 (commit 8f8ae051)
```

A program built from one file (10.9) has the file's name and no version. With `version_flag = true` under `[build]` in `kavach.toml`, the program answers `--version` itself: run with that one argument, it prints the same two lines and ends with exit code 0, before `main` runs. This holds however it runs: `kavach run -- --version`, `run --native` and the built executable. Without the setting, `--version` is an argument like any other, for the program to read.

A built program also carries which runtime operations its compiler knew. Linked with a runtime library from another Kavach (an old copy, or `KAVACH_RUNTIME_LIB` pointing at one), it stops at its start with exit code 2 and a message saying so, instead of calling the wrong operations.

## 10.6 Tests

Testing has its own chapter: [11. Testing](11-testing.md). In short, `kavach test` generates tests from types, contracts and `example` clauses, so few hand-written test files are needed. Hand-written `test` blocks live in `tests/` or next to the code they test (11.8).

## 10.7 Interop

### C (Phase 2)

```kavach
extern c "m" {
    fn cbrt(x: Float) -> Float
}

fn cube_root(x: Float) -> Float {
    unsafe { cbrt(x) }
}
```

`extern c "name"` declares functions of a C library, by their C names and with their C types written as Kavach types. A call is `unsafe` (6.4): it goes inside `unsafe { … }`, so the package needs `unsafe = true`, and the usual way is a safe Kavach function around it, as `cube_root` is here.

- **Types.** Numbers and booleans are passed the same way C passes them: `Int` and `UInt64` are 64-bit integers (`int64_t`, `uint64_t`), `Int32`, `Int16`, `Int8` and the other `UInt` types match their C sizes, `Float` is a `double` and `Bool` a C `bool`. `Ptr<T>` (6.4) is a C pointer `T*`, as a parameter or a result; a `Ptr` to a record is a pointer to a C `struct`. A record itself never goes to C by value. A `String` parameter is a `const char*`: C gets a copy of the text ending in a zero byte, valid only during the call; a text with a zero byte in it panics. A function without `->` returns `void`. A C function can't give a `String`: it gives a `Ptr<UInt8>`, and `p.text()` copies the text (6.4); freeing the C memory, if the library says to, is up to the program. Other types are a compile error for now.
- **Callbacks.** A parameter of a function type, such as `fn(Ptr<Int32>, Ptr<Int32>) -> Int32`, is a C function pointer, with C types only. Its argument is the name of a Kavach function of exactly that type, which can't fail (no `!`): C calls it directly. A lambda or a closure can't be passed, because C has nowhere to keep what it captures. A panic inside the callback ends the program, as it does anywhere else.
- **Libraries.** The library is linked by name: `"z"` is `libz` (`-lz`) on Linux and macOS and `z.lib` on Windows. `"c"` (the C library) and `"m"` (its math part) are always there.
- **Native code only.** C functions run in native code (`kavach build`, `kavach run --native`). The interpreter stops at the first call of one and says to run the program natively.

```kavach
extern c "c" {
    fn llabs(n: Int) -> Int
}
```

```kavach
extern c "c" {
    fn strlen(s: String) -> UInt64
    fn memset(p: Ptr<UInt8>, value: Int32, n: UInt64) -> Ptr<UInt8>
}

fn c_length(s: String) -> UInt64 {
    unsafe { strlen(s) }
}

extern c "c" {
    fn getenv(name: String) -> Ptr<UInt8>
}

fn home() -> String? {
    unsafe { getenv("HOME").text() }      // `none` when C gives a null pointer
}

extern c "c" {
    fn qsort(items: Ptr<Int32>, count: UInt64, size: UInt64, compare: fn(Ptr<Int32>, Ptr<Int32>) -> Int32)
}

fn by_value(a: Ptr<Int32>, b: Ptr<Int32>) -> Int32 {
    unsafe { a.read(0) - b.read(0) }
}

fn sort(items: Ptr<Int32>, count: UInt64) {
    unsafe { qsort(items, count, 4, by_value) }     // C calls `by_value`
}
```

### Python and JavaScript (Phase 3)

```kavach
extern python "numpy" as np
extern js "lodash" as _
```

Calls into Python or JS need the capability `Foreign`, because Kavach cannot check what foreign code does.

## 10.8 Editions

Kavach promises: **code that compiles today keeps compiling.**

To improve the language without breaking that promise, changes that would break old code are released in a new **edition**, chosen per package:

```toml
edition = "2026"
```

- Packages on different editions work together in one program.
- The editions are `2026`, the first; no edition is the newest. An edition Kavach doesn't have (`"2062"`) is an error that names the ones it has.
- `kavach fix --edition 2029` upgrades code automatically.
- Editions come out rarely (every few years) and only with a migration tool.

## 10.9 Running code quickly

Kavach compiles to native machine code, but trying code should feel as quick as a scripting language.

| You want to run | Do this |
|---|---|
| One file | `kavach run hello.kav`: compiles it in memory with the fast backend and runs it. No project needed (1.3). |
| One file, again on every save | `kavach run hello.kav --watch` |
| One file, as a program to ship | `kavach build hello.kav`: writes the executable `hello` (`hello.exe` on Windows) in the current folder. `-o path` names it. |
| One function | `kavach run orders.kav --fn total_paid`: runs a function that takes no arguments |
| A few lines, interactively | `kavach repl`: type code, see each result at once. Earlier lines stay defined. |
| One expression | `kavach eval "[1, 2, 3].sum(n => n)"` prints `6` |
| A function's examples | `kavach test orders.kav --fn total_paid` runs its `example` lines and generated tests (chapter 11) |

In the editor (Phase 2, with the language server):

- A **▶ Run** button appears above `fn main`, every `test` and every `example` line.
- **Run selection** runs the selected lines in a REPL and shows the result next to the code.
- Output of `print` appears in the editor's output panel, not a separate terminal.

The REPL and `--watch` use the same compiler as `kavach build`, so code behaves exactly the same when it ships.

## 10.10 A library for other languages

`kavach build --lib` compiles a library package (`src/lib.kav`, 10.3) into a shared library that other languages load through C: Node.js (Node-API, `ffi`), Java (JNI, the FFM API), Go (cgo), C# (P/Invoke), Python (`ctypes`), C and C++ ([decision 0026](../decisions/0026-a-library-for-other-languages.md)).

```kavach
// src/lib.kav of the package `geometry`
pub fn area(width: Float, height: Float) -> Float {
    width * height
}
```

`kavach build --lib` writes `.kavach/lib/geometry.dll` on Windows, `libgeometry.so` on Linux or `libgeometry.dylib` on macOS, and the header `geometry.h`:

```c
#include "geometry.h"

double a = geometry_area(2.0, 3.5);     // 7.0
```

- **What is exported.** Every `pub fn` of `src/lib.kav` whose parameters and result are numbers, `Bool`, `Ptr` (10.7) or `String`, under the C name `<package>_<fn>`; a `-` in the package name becomes `_`. Other `pub` functions are listed as not exported, with the reason. They stay usable from Kavach.
- **Text.** A `String` parameter is a `const char*`: UTF-8 ending in a zero byte, copied when the call starts; a null pointer or bytes that aren't UTF-8 panic. A `String` result is a new `char*` of the same form, which the caller frees with `<package>_free`, a function every library exports (C's `free` may belong to another C library than the one Kavach's library uses).
- **Errors.** A function that can fail (`-> T ! E`) returns an `int32_t` instead: 0 when it succeeded, and then its value is written to one more parameter, `T* result` (left out when there is no value; `result` may be null when the caller doesn't need it). When it failed, the number is the error's variant, counting from 1 in the order the union declares them, or 1 for an error that isn't a union. The header names each number, and `<package>_last_error()` gives the error as text, as Kavach prints it (`TooOld(age: 200)`), to free with `<package>_free` (null before any error).

  ```kavach
  pub type AgeError = Negative | TooOld(age: Int)

  pub fn check_age(age: Int) -> Int ! AgeError {
      if age < 0 { fail AgeError.Negative }
      if age > 150 { fail AgeError.TooOld(age: age) }
      age
  }
  ```

  ```c
  #define PEOPLE_AGE_ERROR_NEGATIVE 1
  #define PEOPLE_AGE_ERROR_TOO_OLD 2
  int32_t people_check_age(int64_t age, int64_t* result);

  int64_t age;
  if (people_check_age(200, &age) == PEOPLE_AGE_ERROR_TOO_OLD) {
      char* why = people_last_error();     // "TooOld(age: 200)"
      people_free(why);
  }
  ```
- **The header** declares each exported function with C types: `int64_t` for `Int`, `double` for `Float`, `bool` for `Bool`, `T*` for `Ptr<T>`, `void*` for a pointer to a record, and `const char*` and `char*` for text going in and out.
- **Calls** may come from any thread, and from several at once ([decision 0035](../decisions/0035-library-calls-overlap.md)). Each call computes at the same time as the others until its first effect another call could see (printing, logging, files, the network, tasks, channels, a panic); there it waits for the library's lock, and keeps it until it returns. So what different calls print and do never interleaves. A C callback that calls back in from the same call goes on instead of waiting. `<package>_last_error()` gives the last error of the calling thread. C functions the library calls (10.7) may run in several calls at once. The first call sets up Kavach's runtime; what an exported function prints is written out before it returns. A panic prints its message and ends the process, as in a program (5.8).
- **Native code only:** the library is compiled code, like the executables of `kavach build`.

## 10.11 Settings reference

Every setting a program can have, in one place. A key or section that isn't listed here is an error in `kavach.toml`, so a misspelled setting (`taks = "threads"`) can't be ignored silently; the error names the keys the section has.

### kavach.toml

| Section | Key | Values | Default | Spec |
|---|---|---|---|---|
| `[package]` | `name` | text | `""` | 10.4 |
| `[package]` | `version` | text | `""` | 10.4 |
| `[package]` | `edition` | text, such as `"2026"` | `""` | 10.8 |
| `[package]` | `unsafe` | `true`, `false` | `false` | 6.4 |
| `[build]` | `version_flag` | `true`, `false` | `false` | 10.5: the program answers `--version` itself |
| `[limits]` | `file_lines` | a whole number | `500` | 12.1 |
| `[limits]` | `fn_lines` | a whole number | `80` | 12.1 |
| `[limits]` | `level` | `"error"`, `"warn"`, `"off"` | `"error"` | 12.1 |
| `[limits]` | `include` | a list of file patterns | `["**/*.kav"]` | 12.1 |
| `[limits]` | `exclude` | a list of file patterns | `[]` | 12.1 |
| `[limits.files]` | a file's path | a whole number: that file's own limit | none | 12.1 |
| `[test]` | `cases` | a whole number | `200` | 11.9 |
| `[test]` | `deep_cases` | a whole number | `10000` | 11.9 |
| `[test]` | `time_budget` | a time per call: `"500ms"`, `"1s"`, `"2m"` | `"1s"` | 11.9 |
| `[ai]` | `provider` | `"anthropic"` | `"anthropic"` | 8.4 |
| `[agents]` | `folder` | a folder's path | `"agents"` | 8.5: each agent's state, journal and audit log go in `<folder>/<Name>/`; the rest of an agent's settings are in its code (`settings`) |
| `[log]` | `level` | `"debug"`, `"info"`, `"warn"`, `"error"` | `"info"` | 14.12 |
| `[log]` | `console` | `true`, `false` | `true` | 14.12 |
| `[log]` | `file` | a file's path | none | 14.12 |
| `[log]` | `max_bytes` | a whole number | `10000000` | 14.12 |
| `[log]` | `keep` | a whole number | `5` | 14.12 |
| `[log.levels]` | a module or event type | a level, as in `[log]` | none | 14.12 |
| `[log.otlp]` | `endpoint` | a URL; required in this section | none | 14.12 |
| `[log.otlp]` | `service` | text | the package's `name` | 14.12 |

Paths are relative to the folder `kavach.toml` is in. `[dependencies]`, `[build.release]`, `[workspace]` and `[architecture]` are accepted and described where they are used (10.4), but not read yet.

### Environment variables

For running programs:

| Variable | Values | What it does | Spec |
|---|---|---|---|
| `KAVACH_GPU` | `off`, `on`, `check` | Where `map` with a `gpu fn` runs | 15.2 |
| `ANTHROPIC_API_KEY` | a key | The key `ask` uses; never in `kavach.toml` | 8.4 |
| `KAVACH_AI_URL` | a URL | Where `ask` sends its requests (a test server) | 8.4 |
| `NO_COLOR` | any text | No colors in what the program prints | 14.12 |
| `KAVACH_CHECK_LEAKS` | any value | At the end, report heap objects that were never freed | 6.1 |
| `KAVACH_BACKEND` | `cranelift` | `kavach build` uses Cranelift instead of LLVM | 10.5 |
| `KAVACH_LLVM_BIN` | a folder | Where LLVM's tools are | 10.5 |
| `KAVACH_RUNTIME_LIB` | a file's path | Where the runtime library is, for linking | 10.5 |
| `KAVACH_SERIAL` | any value | Loops that would run on all cores run on one | 9.3 |
| `KAVACH_PROCESS_CALL` | a number | Set by `spawn process` for the program it starts: run only that call; not for setting by hand | 9.8 |

For work on the compiler (they print what a pass did, or turn one off):

| Variable | What it does |
|---|---|
| `KAVACH_CLIF` | Prints the Cranelift code of each function |
| `KAVACH_LLVM_IR` | Prints the LLVM code before LLVM optimizes it, and why LLVM isn't used |
| `KAVACH_NO_SROA` | Turns off splitting records into one local per field |
| `KAVACH_SROA_ONLY` | Splits records only in the functions named (comma-separated) |
| `KAVACH_SROA_LOG` | Prints each record split |
| `KAVACH_FUSE_LOG` | Prints each pair of loops fused into one |
| `KAVACH_FUSE_SKELETON` | Prints `main`'s loops before and after fusing |
| `KAVACH_PAR_LOG` | Prints each loop spread over all cores |
| `KAVACH_UNRECURSE_LOG` | Prints why a recursion did or didn't become a loop |
| `KAVACH_FUZZ_CASES` | How many cases the fuzz tests try |
| `KAVACH_COMMIT` | When the compiler is built: the commit `kavach --version` and built programs name (10.5), where git can't tell it |

## Open questions

- Registry name and URL (e.g. `hub.kavach.io`), and package signing.
- Should the dependency manifest record which capabilities each dependency is allowed to use?
