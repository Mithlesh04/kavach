# 10.5 Command line

| Command | Does |
|---|---|
| `kavach new my-app` | Create a project |
| `kavach run` | Compile quickly and run (a project, or one file: 10.9) |
| `kavach run --dry-run` | Run without executing capability calls |
| `kavach run --watch` | Run, and run again each time a `.kav` file or `kavach.toml` of the project changes: the running program is stopped first |
| `kavach run --hot` | Run, and swap in each function changed in a `.kav` file while the program goes on, keeping its values (below) |
| `kavach build` | Compile to an executable file (a project, or one file: 10.9) |
| `kavach build --release` | Optimized build |
| `--env-file <path>`, `--env NAME=value`, `--no-env-file` | On `run`, `test` and `build`: which `.env` file and which single variables the program gets (10.13) |
| `kavach build --lib` | Compile a library into a shared library with a C header, for other languages (10.10) |
| `kavach build --compress` / `--compress=max` | Write a smaller executable, packed without losing anything; it unpacks itself when it starts (below) |
| `kavach run --tracy`, `kavach build --tracy` | Run or build the program with Tracy's client, so the Tracy profiler shows its spans, tasks and GPU work live (14.12) |
| `kavach run --watch-nan`, `kavach build --watch-nan` | Run or build the program with the NaN watch: the first float operation that gives `NaN` panics, naming it (5.9) |
| `kavach test` | Run generated and hand-written tests (chapter 11) |
| `kavach fmt` | Format all code |
| `kavach add <name> --path <folder>` / `kavach add <name> --git <url> [--tag T \| --branch B \| --rev C] [--unsafe]` | Add a dependency, fetch it, write the capabilities it uses into its `can` and write `kavach.lock` (10.12) |
| `kavach update` | Fetch every git dependency again at the tag, branch or commit `kavach.toml` asks for, and rewrite `kavach.lock` (10.12) |
| `kavach audit` | List unsafe code, revealed secrets and capability use across all dependencies |
| `kavach approvals` | List the requests a project's agents wait on: number, call, role, when asked, until when (8.3) |
| `kavach approve <id> --as <name>` / `kavach deny <id> --as <name> [--reason <text>]` | Answer one; the program waiting sees it within a second, or the next start finds it (8.3) |
| `kavach proto` | Write the `.proto` file for a program's types, so other languages' gRPC tools can talk to it (14.14) |
| `kavach fix` | Apply safe fixes: compiler suggestions, deprecations (12.6), edition upgrades (10.8) |
| `kavach api check` | Find breaking changes in `pub` signatures before a release (12.5) |
| `kavach repl` / `kavach eval` | Try code interactively or evaluate one expression (10.9) |
| `kavach --version` | Print the compiler's version and the commit it was built from: `kavach 0.1.0 (commit 8f8ae051)` |
| `kavach version <program>` | Print what built an executable: its package's name and version, the compiler's version and commit, and when, for which system and with which options it was built (below) |
| `kavach lsp` | Language server for editors, on stdin and stdout: errors as you type, with the same messages as `kavach check`, checked file by file (13.12); the type of what is under the cursor (hover); where a name is declared (go to definition); and the names in scope, or after a `.` the fields and methods of the value (completion) |
| `kavach <command> --help` | Print that command's usage (`-h` too); `kavach --help` prints them all |

Not built yet: `api check`, `repl` and `eval` (Phase 3 tools, [roadmap](../../vision/roadmap.md)), and `build --release`: `kavach build` already optimizes. Until then they print the command list. `kavach fix` works on the files and folders it is given (`kavach fix src`); with none, it doesn't yet fix the whole project, and edition upgrades wait for a second edition (10.8).

`kavach new my-app` creates the folder `my-app` with the layout of 10.3: a `kavach.toml`, a `src/main.kav` that prints a greeting, and a `.gitignore` for `.kavach/`. The name must be lowercase `kebab-case`.

Inside a project (a folder with `kavach.toml`, or any folder below it), `run`, `build`, `test` and `check` without a file work on the project: its `src/main.kav`. Arguments after `--` go to the program: `kavach run -- --verbose`.

`kv` is a short alias for `kavach`.

**A program's version.** Every executable `kavach build` writes carries, as text inside it, its package's `name` and `version` (10.4), the version and commit of the compiler that built it, and how it was built. `kavach version` prints them, so a program found on a server, or the program that trained a model, can be traced to its source and its build:

```
$ kavach version .kavach/bin/my-app
my-app 0.1.0
built by kavach 0.1.0 (commit 8f8ae051)
built 2026-11-23T09:41:07Z for x86_64-linux with --tracy
```

- The third line gives when `kavach build` ran, as a moment in UTC; the system the program runs on, as `<processor>-<system>` (`x86_64-windows`, `aarch64-macos`); and the options that change what is built: `--tracy`, `--watch-nan` (given, or set in `kavach.toml`), `--compress` or `--compress=max`, and `KAVACH_BACKEND=cranelift` (10.11). Without them, the line ends after the system.
- With `SOURCE_DATE_EPOCH` set to a whole number of seconds since 1970, as tools for repeatable builds set it, the moment is that one instead of the clock's, so building the same source again doesn't change the text.
- `kavach run` keeps no executable, so a program run that way has the first two lines only.

`kavach run --watch` looks at the project's `src/` folder (or the folder of the one file given) and its `kavach.toml` a few times a second. When one changes, it stops the running program, printing `-- changed: src/orders.kav, running again`, and starts it again; a program that ended waits for the next change. It stops when you press Ctrl+C. It is for working on a program, not for running one in production.

`kavach run --hot` runs the program in the interpreter, and when a `.kav` file changes, checks the program again and swaps in its new functions without stopping it, keeping every value it has: a game keeps its level, its player and its window while you change how it moves. Each call made after the change runs the new code; code already running goes on as it was, so a loop in `main` that calls `update(state)` each frame runs the new `update` from the next frame on.

```
$ kavach run --hot game.kav
-- changed: game.kav, functions swapped
```

- Only function bodies may change. When a type, a function's parameters or result, an agent, a capability, a `use` or the standard modules the program uses change, the values can't be kept: the program stops and runs again from its start, as `--watch` does, printing `-- changed: game.kav, the program's types changed: running again`.
- A change with errors is shown, and the program goes on with the code it had.
- Tasks already running go on with their code; tasks started later run the new code.
- It is for working on a program, and runs it in the interpreter only.

A program built from one file (10.9) has the file's name and no version. With `version_flag = true` under `[build]` in `kavach.toml`, the program answers `--version` itself: run with that one argument, it prints the same lines and ends with exit code 0, before `main` runs. This holds however it runs: `kavach run -- --version`, `run --native` and the built executable. Without the setting, `--version` is an argument like any other, for the program to read.

**An icon and a manifest on Windows.** On Windows, `kavach build` gives the executable a manifest: it runs as the user who starts it, with no administrator prompt; it is sharp on high-DPI screens (per-monitor DPI awareness); and it may use paths longer than 260 characters. With `icon = "assets/icon.png"` under `[build]` in `kavach.toml`, the executable also has that icon, which Explorer and the taskbar show:

```toml
[build]
icon = "assets/icon.png"      # a PNG of at most 256 × 256 pixels
```

The path is relative to `kavach.toml`. A file that isn't a PNG, or one larger than 256 × 256, is an error that says so. Kavach writes the Windows resources itself, so no Windows SDK is needed ([decision 0120](../../decisions/0120-windows-icon-and-manifest.md)). Other systems give a program its icon outside the executable (a `.desktop` file on Linux, an app bundle on macOS), so there the setting is checked but not used.

**No console window on Windows.** A program that shows a window doesn't want a console window beside it. With `console = false` under `[build]`, `kavach build` makes it a Windows window program: it starts with no console window, and what it prints goes nowhere, as nothing shows it. A panic still ends it, and goes to the crash reports of `std.app` when the program asks for them (14.30). The default is `true`. Other systems have no such kind of program, so there the setting is checked but not used.

```toml
[build]
console = false
```

**A packed executable.** `kavach build --compress` writes the program packed, with nothing lost: a little under half its size (deflate), or about a third to two fifths with `--compress=max` (xz). Run, it unpacks itself into a cache folder and runs the unpacked copy, which behaves exactly as the program built without `--compress`: the same output, exit code, arguments and environment, and the same messages for panics and errors. The copy is named by a hash of its contents, so the first start unpacks it (a few milliseconds with `--compress`, tens of milliseconds with `--compress=max`) and later starts run it at once; deleting the cache folder is always safe. The folder is `KAVACH_UNPACKED` when set, else `%LOCALAPPDATA%\Kavach\unpacked` on Windows, `~/Library/Caches/kavach/unpacked` on macOS, and `$XDG_CACHE_HOME/kavach/unpacked` (or `~/.cache/kavach/unpacked`) elsewhere. `kavach version` reads a packed program too.

```
$ kavach build --compress hello.kav -o hello
$ ./hello
hello
```

Some antivirus tools distrust programs that unpack themselves, which is why packing is asked for, not done by default.

A built program also carries which runtime operations its compiler knew. Linked with a runtime library from another Kavach (an old copy, or `KAVACH_RUNTIME_LIB` pointing at one), it stops at its start with exit code 2 and a message saying so, instead of calling the wrong operations.
