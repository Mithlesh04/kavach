# Tooling review

A gap list for Kavach's tools: step 1, item 4 of the [to-do](../vision/todo.md) (foundation 4). It checks the command line, the language server, the editor extension, the formatter, the test runner, the package manager, `kavach audit`, the AI tools, the playground, and whether a debugger and a profiler exist. Each gap names the test that would show it closed.

It sits beside [compiler-review.md](compiler-review.md) and [checker-review.md](checker-review.md) and does not repeat them. Where a gap is already there, this file links to it (R1–R7 are the compiler review's rewrites).

**How it was made (2026-11-27, at commit `872dc9507`).** Source files, READMEs and the spec were read. Small probes were run with the installed release, `kavach 0.2.0 (commit 369e1dbb)`, on files in a temporary folder. Nothing was built. A finding marked **confirmed** was seen in a probe; **from reading** was not run. Times were taken once, on a loaded Windows machine; all other speed claims are **not measured**.

## Summary

The tools cover a lot: about 20 commands, a language server with quick fixes and file-by-file checks, an MCP server, Tracy and Chrome traces. The gaps are mostly in how the parts fit together, and in two missing tools.

### Top 10 gaps, most visible to users first

1. **VS Code shows half of what the language server offers.** The extension never asks for quick fixes or semantic colors, never sends "file saved", drops each message's code, and has no way to restart the server (confirmed by reading `editors/vscode/extension.js`).
2. **The language server dies on programs `kavach check` accepts.** Its checking thread has the default 2 MB stack, while the CLI gives the compiler 256 MB. A file with 2,000 nested parentheses checks fine with `kavach check` and kills `kavach lsp` with a stack overflow (confirmed). Any panic in a check also kills it, and nothing restarts it.
3. **`kavach mcp` gives false errors in every project.** It checks one file with default options, ignoring the project's other files, `kavach.toml` and dependencies. In a two-file project, `kavach check` says "no errors" and MCP's `kavach_check` says "there is no module `money`" (confirmed).
4. **`kavach fmt` changes code, and ignores mistyped flags.** It renames names in the wrong style, which spec 12.7 says the formatter never does ("never what is on them"), while spec 1.5 says it does. Run on a naming conformance test, it fixes the very mistake the test checks for (confirmed). `kavach fmt --chek f.kav` formats the file and exits 0 (confirmed).
5. **There is no debugger, and no debug information.** `kavach build` writes no line tables. The error trace that spec 5.8 promises "in debug builds" is not built (confirmed: only `error: Empty` is printed), and Kavach has no debug builds at all, though spec 5.8 and 7.4 name them.
6. **There is no function-level profiler.** Tracy and `KAVACH_TRACE` show only spans the program writes itself. Built programs carry no symbols: Linux links with `-s`, and Windows writes no PDB. So `perf`, `samply`, Windows Performance Analyzer or VTune can't name a Kavach function (from reading `link.rs:146`).
7. **The command line handles arguments differently per command.** An unknown command prints the whole help with no message. Flags must come before the file (`kavach check f.kav --format json` fails). `--tracy` is taken even from the program's own arguments. `kavach add`, `kavach update` and the `--env` flags are missing from `--help` (all confirmed or read in `main.rs`).
8. **Error messages and their fixes are uneven.** Codes come from the wording (compiler review R6). The crash-fix advice for an overflow in `n * 2` on an `Int` says "use `Int` instead of `Int32`" and "use `checked_add`" (confirmed). One mistake still gives two messages in places, and `?` (an unknown type) reaches users in messages and hover (confirmed).
9. **`kavach audit` can't gate a release.** It always exits 0, has no JSON form, lists every standard library function that uses a capability whether the program calls it or not (confirmed), and `audit --panics` (spec 11.10) is not built.
10. **The test runner is interpreter-only and runs on one thread.** It has no way to pick tests by name, no machine-readable report, and no `--changed`, `--suggest` or `fix … --option` (spec 11.9, 11.7, 11.10).

## 1. The command line as a whole

**Spec:** 10.5 (the command table), 10.9, 10.13, 10.14. **Code:** `tools/cli/src/main.rs` matches the argument list against fixed patterns; `help.rs` holds the help text.

**What works.** Every command in 10.5 except `api check`, `repl`, `eval` and `build --release` exists, as 10.5 says. Inside a project, `run`, `build`, `test`, `check`, `audit` and `proto` find `src/main.kav`. `kavach <command> --help` prints that command's lines. Messages that name a spec section are common and helpful.

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| An unknown command (`kavach chek f.kav`) prints the whole help to stderr, exit 2, with no "unknown command" line or suggestion | confirmed | `tools/cli/tests/usage.rs`: stderr starts with ``error: unknown command `chek`; did you mean `check`?`` |
| Flags only work before the file: `check f.kav --format json`, `build -o out f.kav` print the whole help | confirmed; `main.rs:88-224` | the same tests accept flags anywhere before `--` for commands that take no program arguments |
| `--tracy` is removed from anywhere in the arguments, even after `--`, so a program can't receive it; `--watch-nan` correctly looks only before the file | from reading `native.rs:50-60` | `kavach run app.kav -- --tracy` prints `--tracy` from `cli.args()` |
| `run --dry-run --native f.kav` says "can't read --native" | confirmed | a clear "`--dry-run` runs in the interpreter only" error, exit 2 |
| `--help` lacks `add`, `update`, `--env-file`, `--env`, `--no-env-file`; `kavach add --help` falls back to the whole list | `help.rs` | a unit test: every command `main.rs` matches has its own help lines, and every 10.5 row is in the help or listed as not built |
| `kavach audit` outside a project prints the help with no message | confirmed | "give a file, or run it inside a project", as `fmt` says |
| Exit codes are listed only in `tools/cli/README.md`, not in 10.5; a failed git fetch exits 2 ("wrong usage"); a compiler panic exits 101 like a program's panic | `deps.rs`, `main.rs:53` | 10.5 lists the codes; a test per code |
| `cargo install --path tools/cli` installs no `kv`, which 10.5 promises; only the release archives have it | `tools/cli/Cargo.toml` | a second `[[bin]]`, or 10.5 says the alias comes with the installers |

**Docs drift** (not in the compiler review's list): `tools/README.md` says the language server has "hover and go-to-definition next", lists an empty `pkg/` folder as the package manager, and its status line misses most commands. `tools/cli/README.md` misses `audit.rs`, `env.rs`, `pack.rs`, `proto.rs`, `approvals.rs`, more than 20 test files, and the commands `add`, `update`, `fix`, `fmt`, `audit`, `proto`, `approvals`. `tools/lsp/README.md` still says "Next: a VS Code client". `tools/ai/README.md` says "fixes inside messages" are next, but `json.rs` already writes them.

**Speed:** `kavach --version` took 144–171 ms (3 runs), and `kavach check` of a 10-line file about 200 ms. Most of that is starting a 45 MB executable, not checking. This matters for tools that call `kavach` per file (editors without the language server, scripts, the MCP client). Not measured: what the start-up time is spent on.

## 2. The language server (`kavach lsp`)

**Spec:** 13.12, 10.5. **Code:** `tools/lsp/` (3,800 lines, 15 files).

**What works.** Errors as you type with their codes, checked on a thread of its own and coalesced; file-by-file checks after a body change (13.12); hover with the type; go to definition across files; completion of names and members; semantic tokens; quick fixes, shared with `kavach fix`. The in-crate tests drive whole conversations, and `quick/tests.rs` compares quick checks with whole ones on every conformance project.

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| The checking thread uses the default 2 MB stack (`checking.rs:41`); a deep program kills the server | confirmed: exit `0xC00000FD` on 2,000 nested parentheses | `tools/lsp/src/tests.rs`: that file gets its diagnostics |
| A panic in a check ends the server; there is no `catch_unwind`, so no message is published and the editor's requests hang or fail | from reading | a test with a fake check that panics: an "internal error" diagnostic is published and the server keeps answering |
| A file not yet saved (untitled) gets no diagnostics at all (`lib.rs:331`) | from reading | a `didOpen` with an `untitled:` URI gets diagnostics, checked as a single file |
| Completion compiles the whole program on the message thread for every request (`complete.rs:49-51`); hover, definition and colors do too after any edit (`lib.rs:307-321`) | from reading | not measured; a test that completion on a 50-file project answers within a set time, once a timer exists (compiler review cheap fix 9) |
| No references, rename, document symbols, workspace symbols, signature help, formatting or inlay hints; `kavach outline` and the index (13.9) already hold what symbols and references need | `initialize` result, `lib.rs:153-166` | each provider in `initialize`, with a conversation test |
| Hover shows only the type: no doc comment, no `uses`, `!` error or contract, which are what Kavach signatures are for (13.3); on a keyword it shows `example: ?` | confirmed | hover on a call shows its outline line (`kavach_ai::outline_item`) |
| Files changed on disk but not open (a `git checkout`, `kavach add`) are not seen until the next whole check; `workspace/didChangeWatchedFiles` is not handled | from reading | a test: a closed file changed on disk updates the open file's diagnostics |
| Requests can't be cancelled (`$/cancelRequest` is ignored) | from reading | low priority until completion is slow (measure first) |

**Speed:** not measured. A whole check is a full compile with the standard library, with nothing cached (compiler review, section 4). Requests also call `canonicalize` (a system call) for each file they compare or look up (`navigate.rs:21`, `lib.rs:429`).

## 3. Editor support

**Spec:** 13.12 (through the server). **Code:** `editors/vscode/` (a 119-line extension, a 93-line JSON-RPC client, two grammars).

**What works.** Highlighting of `.kav` files and of ```` ```kavach ```` blocks in Markdown; errors, hover, definition and completion through the server; no npm packages needed; a client test that drives the real server.

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| Quick fixes are never asked for: no `registerCodeActionsProvider` | `extension.js` | `client.test.js` asks for code actions through the same calls the extension makes; a manual check in VS Code |
| Semantic tokens are never asked for, so the colors set in `package.json` (`*.imported`, `method.defaultLibrary`) never apply | `extension.js`, `package.json` | the same, for `semanticTokens/full` |
| "File saved" is never sent, so the whole-program check that 13.12 promises on save never happens in VS Code | `extension.js` | `client.test.js`: after `didSave`, another file's diagnostics change |
| Each diagnostic's code (13.6, 13.12) is dropped; information and hint severities show as errors | `extension.js:43-50` | the diagnostic carries `code`; severity 3 shows as information |
| The server's stderr is piped but never read; if it fills the pipe, the server blocks | from reading `client/rpc.js` | stderr goes to a "Kavach" output channel |
| A server that stops stays stopped; no restart command, no message | `client/rpc.js` | a "Kavach: Restart language server" command; a stopped server starts again once |
| Every change sends the whole text, though the server takes incremental edits | `extension.js:75-80` | low priority; measure on a large file first |
| No commands to run, test or format the current file, and no `kavach.toml` schema | `package.json` | commands `kavach.run`, `kavach.test`, `kavach.format` |
| No other editor: no Neovim, Helix, Zed or JetBrains setup, and no tree-sitter grammar (which those editors and GitHub use) | `editors/README.md` | a page with the LSP setup for Neovim and Helix (they need no extension) |
| No test keeps the grammar in step with the lexer's keywords; today all 45 keywords are in it (checked by a script) | none exists | a Rust test reading `token.rs` and the grammar JSON |
| The extension's comment cites spec 10.4 for the server; it is 10.5 and 13.12 | `extension.js:5` | none needed |

The grammar tests and `client.test.js` run only by hand (CI is off). The repo check does not run them.

## 4. The formatter (`kavach fmt`)

**Spec:** 12.7 (layout only, no options, `--check`), 1.5 (fixes names). **Code:** `tools/fmt/` (792 lines), `tools/cli/src/fmt.rs`.

**What works.** One style, no options. After layout, the tokens must be unchanged or nothing is written. Tests check idempotence and that every `.kav` file under `examples/` and `tests/conformance/` is formatted.

**Is renaming allowed?** The spec disagrees with itself. Spec 1.5 says `kavach fmt` renames names in the wrong style with every use. Spec 12.7 says the formatter "sets only the layout of lines, never what is on them". The code follows 1.5 (`fix_names` runs first). The side effect: `kavach fmt tests/conformance/01-basics/variables-use-snake-case.kav` turns `orderCount` into `order_count`, so the test no longer tests anything (confirmed on a copy). Inside a project, plain `kavach fmt` also formats `tests/`, so any test with a deliberately wrong name changes. A style mistake is a compile error, so it has a message with a fix already. **Recommended:** the formatter does layout only, and renaming becomes a fix that `kavach fix` applies (it has one right answer unless the new name is taken, which `fix_names` already checks). That needs a maintainer's choice and a change to spec 1.5 (decision T4 below).

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| The 1.5 / 12.7 contradiction, and conformance tests rewritten | confirmed | `kavach fmt` on a naming conformance test leaves it unchanged |
| Unknown flags and non-`.kav` arguments are ignored: `--chek` formats, `kavach fmt src` formats the whole project instead of `src` | confirmed (`fmt.rs:30-35`) | an unknown flag exits 2 and writes nothing; a folder is formatted like `kavach fix src` does |
| No standard input mode (`kavach fmt -`) and no LSP formatting, so editors can't format on save | `fmt.rs` | `kavach fmt -` prints the formatted text; LSP `textDocument/formatting` |
| `fix_names` compiles each file alone with default options (`names.rs:30`), not as part of its project; a file that uses another module gets errors, and names it could fix may be left | from reading | in a project, a private wrong name in a file that uses another module is fixed |
| No CLI test runs `kavach fmt` or `kavach fmt --check` (exit codes, project mode) | `tools/cli/tests/` | a `tools/cli/tests/fmt.rs` |

**Speed:** not measured. Each file is compiled in full (with its standard library modules) before layout, only to find names. A project of 100 files is 100 compiles. With renaming moved to `fix`, `fmt` needs only the lexer.

## 5. `kavach fix` and error messages

**Spec:** 12.7, 13.6, 11.10, 5.8. **Code:** `tools/lsp/src/fixes.rs`, `suggest.rs`, `deprecated.rs`; `compiler/driver/src/codes.rs`; `compiler/interp/src/testing/fixes.rs`.

**What works.** `kavach fix <files or folders>` applies the safe fixes 12.7 lists, and leaves out any that adds an error. `check --format json` carries `fix` with `safe` (13.6). A file with syntax errors shows only those (13.6).

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| Codes come from the wording (`codes.rs:233-237`); "there is no module `money`" gets `check`, though 13.6 gives `unknown-name` for "isn't a module" | confirmed | compiler review R6 |
| One mistake, two messages: a missing module gives "no module" and "`money` is not defined"; `-> Unit` gives "`Unit` is not a known type" and "`main` must return `?`" | confirmed | each case as a conformance test with exactly one marked error (compiler review cheap fix 2) |
| `?` (the checker's unknown type) reaches users: in "`main` must return `?`" and in hover | confirmed | a checker test: no message text contains `` `?` `` |
| The 13.6 table lists `style` under warnings; a wrong-style name is an error | confirmed | the table or the severity changed to match |
| Crash-fix advice for overflow is fixed text: "use `Int` instead of `Int32`" when the type is already `Int`, and `checked_add` for a `*` | confirmed (`fixes.rs:44-49`) | the advice names the real operator (`checked_mul`) and leaves out "wider type" for `Int` |
| The overflow message shows the first failing value, the "smallest failing input" line another (`-9223372036854775808 * 2` against `n = -4611686018427387905`) | confirmed | the message is from the shrunk case |
| A type mismatch names the types but not the fix: `const a: [Int] = [2, 3, 4].sum(p => p)` says "expected `[Int]`, found `Int`" (reported by a maintainer, 2026-11-27) | confirmed | the message says `sum` gives one `Int`, the total, and suggests `: Int` or leaving the type out |
| `kavach fix stats.kav:14 --option 1` (11.10) is not built; `fix` takes only files and folders | `main.rs:172` | the 11.10 example works |
| `--context` (13.6: a source excerpt with JSON messages) is not built | `main.rs:147` | `check --format json --context` adds the line |
| `kavach fix` stops at the first file it can't read, leaving the files after it unfixed | `main.rs:259-270` | every file is tried; exit 1 if any failed |
| No way to read more about a code, as `rustc --explain` gives | not in spec | an open question first; the website could hold one page per code |

## 6. The test runner (`kavach test`)

**Spec:** chapter 11 (11.9 commands and settings, 11.7 report, 11.10 crash fixes). **Code:** `main.rs:399-448`, `compiler/interp/src/testing/`.

**What works.** Examples, generated cases from types and contracts, shrinking, found cases replayed from `tests/found/`, `--seed`, `--deep`, `--keep`, a time budget with a watchdog, capability fakes, a "weakly tested" report, examples that reach C listed as not run.

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| Interpreter only: no `kavach test --native`, so C wrappers and native-only behaviour go untested | to-do step 1 item 14 | that item's test |
| One thread: functions and cases run one after another | from reading (no threads in `testing/`) | not measured; a run on a 100-function file, timed with 1 and N threads, same seed, same report |
| No way to run one function's or one test's cases (`kavach test f.kav refund`) | `main.rs:421-439` | a filter, with a test |
| No machine-readable report (JSON lines), so editors and CI can't show results | none | `--format json` with one line per function |
| `--changed`, `--suggest`, `audit --panics`, the "never reached" part of the report | spec 11.11 (Phase 3) | already in the to-do (step 4, item 15); listed so the count is complete |
| The report says "1 functions" and prints "0 passed, 0 failed" when there are no examples | confirmed | the exact report lines in a CLI test |
| `kavach test` on a single file outside a project writes `tests/found/` next to it | confirmed; to-do step 1 item 10 | that item |

**Speed:** not measured. Default 200 cases per function; budget counted in steps, so runs repeat exactly.

## 7. The package manager (`kavach add`, `kavach update`)

**Spec:** 10.12, 10.14, decisions 0083 and 0084. **Code:** `tools/cli/src/deps.rs` (168 lines), `compiler/driver/src/deps.rs`. The `tools/pkg/` folder is empty.

**What works.** Path and git dependencies, `kavach.lock`, `kavach update`, `can` written from what the package uses, `unsafe` refused until allowed, workspaces, safe transports only. So packages need not wait for `kavach add`: it exists and has tests (`tools/cli/tests/packages.rs`), as the to-do's step 4 item 15 records. That same item still lists `kavach fix` and `kavach audit` as Phase 3 tools to build, though both exist; what is left of them is `fix --option` and `audit --panics`.

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| `kavach.toml` is edited as text: the entry goes after the first `[dependencies]` found anywhere, and a name added twice gives a TOML error, not "already added" | from reading `deps.rs:93-100` | `kavach add` twice says the name is there; a comment holding `[dependencies]` is left alone |
| No `kavach remove`, no way to update one package (`kavach update money`), no list or tree of what is used | `main.rs:95-96` | each, with a test in `packages.rs` |
| No `--locked` or offline mode for CI: a build fetches when something is missing | from reading `deps.rs:24-35` | `kavach build --locked` fails instead of fetching |
| Every `run`, `check` and `build` in a project resolves the dependencies first (`fetch_for`) | `main.rs:310` | not measured |
| No registry, versions or signing | to-do step 4 item 15 (Phase 3) | that item |

## 8. `kavach audit`

**Spec:** 10.5 ("across all dependencies"), 11.10 (`--panics`). **Code:** `tools/cli/src/audit.rs` (142 lines), tested in `tools/cli/tests/audit.rs`.

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| Lists every standard library function that uses a capability, called or not (`std/fs.kav:51 fn write: Fs.write` for a program that only reads) | confirmed | only functions the program reaches, grouped by package |
| Always exits 0, and has no JSON form, so CI can't require "no `unsafe`" or "no new capability" | `audit.rs:96` | `--format json`; a flag that fails on any `unsafe` |
| Results are not grouped by dependency, which is what 10.5 asks to see | from reading | the output names each dependency |
| `audit --panics` is not built | spec 11.10, 11.11 | already Phase 3 |

## 9. The AI tools: `outline`, `query`, `context`, `mcp`

**Spec:** 13.2–13.9, 13.11. **Code:** `tools/ai/` (1,086 lines), `tools/cli/src/context.rs`.

**What works.** `query` (callers, uses, fails, type, find, tokens), `outline`, `context` with a budget, the project index in `.kavach/index`, `check --format json` with fixes, `llms.txt` kept under its token limit by a test, and MCP with time limits.

| Gap | Evidence | Test that shows it closed |
|---|---|---|
| MCP compiles one file with `Options::default()` (`mcp.rs:141`): no project files, no `kavach.toml`, no dependencies, no tagged-literal checks; false errors in any project | confirmed | `tools/ai/tests/mcp.rs`: a two-module project checks clean through `kavach_check` |
| MCP needs `file` for every tool; in a project, `query` and `context` don't use the index the CLI keeps | `mcp.rs:139` | MCP's `kavach_query` with no file answers from `.kavach/index` |
| MCP has no `fix` or `fmt` tool; `edit` (13.8) is Phase 3 | `mcp.rs` | a `kavach_fix` tool |
| `kavach outline <module>` (13.3 says "file or module") takes only a path: `outline money` says "can't read money" | confirmed | `kavach outline money` in a project prints that module |
| Each tool puts the compile together its own way: CLI `read_as`, MCP, `fmt`, the playground and the server all differ | `main.rs:305`, `mcp.rs:141`, `names.rs:30`, `playground/src/main.rs:30` | decision T3 |

**Speed:** `query` and `context` from the index are not measured; the index is rebuilt with a whole compile when any file changes (13.9, open question).

## 10. Smaller tools: `svd`, `proto`, `approvals`, `version`

These match their spec sections and have tests (`tools/svd/tests/`, `tools/cli/tests/{registers,proto,version}.rs`, `tests/approvals/`). Gaps:

- `kavach svd` skips clusters and enumerated values, and says so in comments. It is tested on one hand-written chip only; a real vendor file (an STM32's, thousands of registers) has never been run, so its speed and coverage are unknown. Test: one real SVD file from a vendor, checked into `tools/svd/tests/`, compiles.
- `kavach svd` and `kavach proto` print to standard output only; no `-o`. Small.

## 11. `kavach build` and `kavach run` options

**Spec:** 10.5, 10.9, 10.11. **What exists:** `--native`, `--dry-run`, `--watch`, `--hot`, `--tracy`, `--watch-nan`, `--compress`, `--lib`, `-o`, the `--env` flags; `KAVACH_BACKEND` chooses LLVM or Cranelift.

- **One flag per run, in a fixed order.** `--dry-run` and `--hot` are interpreter-only and can't be combined with `--native`; the message for trying is wrong (section 1).
- **No `--release`, and so no debug build.** Spec 5.8 (error trace) and 7.4 (contract checking) describe both kinds; 12.7 adds "no warnings in release builds". `[build.release]` is read but does nothing. Decision T1.
- **No target choice.** `kavach build` builds for the machine it runs on only. Not promised by the spec; worth an open question in 10.5.
- **No build timing or verbose output.** Compiler review cheap fix 9 (`KAVACH_TIMES`).

## 12. The playground

**Spec:** decision 0078 (milestone 1: check only). **Code:** `tools/playground/src/main.rs` (72 lines, 2 tests).

- It checks and does not run, as milestone 1 says; the page says so.
- Its JSON has no `code` and no `fix`, unlike `check --format json` (13.6), so the page can't offer a fix. Test: a message with a known fix has `"fix"` in the answer.
- It compiles with default options and no `kavach.toml`. That is right for samples, but once decision 0148's default capabilities are built, the page should say which settings it checks with.
- The site converts byte offsets to lines correctly (`playground-editor.tsx:61`).
- Speed and size of the WebAssembly file: not measured.

## 13. A debugger

**There is none.** What exists instead: panic messages with a place, crash reports listing the functions that were running (14.30), `--dry-run`, `--hot`, `--watch-nan`, and spans in a trace. Nothing pauses a program, steps through it or shows its variables. `kavach build` writes no debug information (the to-do records that crash reports were built from a table of their own because of this).

The smallest useful versions, in order:

1. **The error trace (spec 5.8).** Each `try` an error passes through adds its place; when `main` returns the error, the places are printed under the message. It is specified, small, and answers "where did this error come from?", the most common question a debugger is opened for. Test: a conformance test in `05-errors/` whose expected stderr holds the trace.
2. **A debugger for the interpreter, over DAP** (the Debug Adapter Protocol, which VS Code, Neovim and JetBrains speak). `kavach run --debug f.kav` serves DAP on stdin and stdout, as `kavach lsp` serves LSP. First requests: set breakpoints by line, continue, pause, step over, step in, step out, the stack, and each frame's variables. The interpreter has most of what it needs: frames with their variables by definition (`interp.rs:16`), and a stop point on every loop round and call (`interp.rs:318`, already used by `--hot` and MCP's time limit). It lacks a hook per statement, and each frame's function and call site. Tasks: pause only the task that hit the breakpoint at first. VS Code needs a `debuggers` entry in `package.json`. Test: a DAP conversation in the style of `tools/lsp/src/tests.rs`: a breakpoint on line 3 stops there, and the variables show `total = 3`.
3. **Line tables in native code** (DWARF on Linux and macOS, CodeView and a PDB on Windows), from KIR's spans, so `gdb`, `lldb` and Visual Studio can step through built programs. LLVM IR can carry `!dbg` locations; Cranelift needs the tables written separately (not checked how much it gives). Decision T2.

## 14. A profiler

**What exists:** `KAVACH_TRACE=file` writes every `log.span` as a Chrome trace with GPU timings (14.12); `--tracy` shows the same live. Both show only the spans a program writes. There is no view of where time goes by function, no allocation profile, and nothing for the interpreter.

The smallest useful versions:

1. **Keep function names in built programs.** On Linux, drop `-s` (or keep the symbol table and drop only debug sections); on Windows, write a PDB with public symbols (`/DEBUG`). Then the existing sampling profilers (`perf`, `samply`, Windows Performance Analyzer, VTune) name Kavach functions, with no new code in Kavach. Cost: a larger file (not measured), so perhaps a `--symbols` flag, or on by default with `--release` stripping them (T1). Test: build a program and find its Kavach function names in the file's symbol table.
2. **`KAVACH_PROFILE=1` in the interpreter:** calls and time per function, printed as a table when the program ends, or written as automatic spans into the same Chrome trace. The call path already passes through `step` (`interp.rs:318`). Test: a program with one slow function lists it first.
3. Later: automatic Tracy zones per function in native code (opt-in), and allocation counts.

## Cheap fixes

Each is small and safe, with the test that shows it done.

1. **The server's checking thread on a big stack, and a panic caught.** Spawn it with the CLI's 256 MB stack and wrap each check in `catch_unwind`, publishing an "internal error" diagnostic. Test: the 2,000-parentheses file in `tools/lsp/src/tests.rs`.
2. **VS Code uses what the server offers:** send `didSave`; register code actions and semantic tokens; keep the diagnostic's code; map all four severities; show the server's stderr in an output channel; add a restart command. Test: `client.test.js` for the requests; a manual check in VS Code.
3. **MCP compiles as the CLI does:** call the CLI's `read_as` path (or its driver equivalent): config, project sources, dependencies, literal checks. Test: a two-module project checks clean in `tools/ai/tests/mcp.rs`.
4. **`kavach fmt` refuses what it doesn't know:** an unknown flag or a non-`.kav` path is an error, exit 2, nothing written; a folder formats that folder. Test: `tools/cli/tests/fmt.rs`.
5. **"Unknown command" says so**, with the closest command, instead of printing the whole help. Test: `tools/cli/tests/usage.rs`.
6. **`--tracy` only before the file**, as `--watch-nan` is. Test: `kavach run app.kav -- --tracy` passes it to the program.
7. **Help for every command:** add `add`, `update` and the `--env` flags; a unit test that each matched command has help lines and each 10.5 row is present or listed as not built.
8. **Crash-fix advice from the real operator and type,** and the overflow message from the shrunk case. Test: the overflow in `n * 2` on an `Int` suggests `checked_mul` and no "wider type".
9. **Report wording:** "1 function", and no "0 passed, 0 failed" line when nothing was written by hand. Test: exact report lines.
10. **`kavach audit` lists only what the program reaches**, and exits 1 under a flag when it finds `unsafe`. Test: `tools/cli/tests/audit.rs`, a program that reads a file lists no `fs.write`.
11. **Untitled files get diagnostics,** checked as a single file. Test: a `didOpen` with an `untitled:` URI.
12. **A grammar test:** every lexer keyword appears in `kavach.tmLanguage.json`.
13. **Tools docs brought up to date:** `tools/README.md` (status, `pkg/`), `tools/cli/README.md` (commands, files, exit codes), `tools/lsp/README.md` ("Next"), `tools/ai/README.md` ("fixes inside messages"), the extension's spec number; and to-do step 4 item 15, which lists `kavach fix` and `kavach audit` as not built. Test: the repo check, plus a reader.
14. **Fix the 13.6 table** (`style` is an error) and give "there is no module" the code `unknown-name` (until R6). Test: `driver/tests/codes.rs` for that message.

## Bigger work worth a decision record

**T1. Debug and release builds.** What "a debug build" is, since spec 5.8 and 7.4 rely on it: every contract checked, the error trace, line tables and symbols. What `kavach build --release` is: `pub` contracts only (7.4), warnings as errors (12.7), symbols stripped or kept. Whether `kavach build` stays optimized by default. *Unblocks:* the error trace, `[build.release]`, profiling with symbols.

**T2. A debugger.** DAP for the interpreter first (section 13, step 2): about 2–3 weeks from reading (not estimated in detail); a per-statement hook costs interpreter speed, to measure. Native line tables second, per backend. Which tasks pause. A `debuggers` entry in the VS Code extension.

**T3. One way for every tool to compile** (extends compiler review R5). The CLI, the language server, MCP, `fmt`, `fix` and the playground each assemble the compile differently today, and MCP gives false errors because of it. One driver call, taking a path, open editor texts and a flag for "this is a single file", that reads `kavach.toml`, the project, dependencies and literal checkers. *Unblocks:* cheap fix 3 for good, `fmt` in projects, the same messages everywhere (13.12 promises this).

**T4. What the formatter may change.** Settle spec 1.5 against 12.7. Recommended: layout only in `fmt`; name fixes in `kavach fix` and as a quick fix. A maintainer's choice, since it changes a language promise.

**T5. One argument parser for the CLI.** Flags in any order before `--`, unknown flags refused, per-command help and the 10.5 table generated from one list. Either Kavach's own small parser or `clap` (an outside dependency, so a decision record either way).

**T6. Language server speed.** Every request answered from a background compile, never on the message thread; caching the standard library's tree (compiler review section 4, step 2); references, rename and symbols from the index (13.9). Depends on R1 and R7 for real incremental checks; measure first with a timer.

## What this review could not check

- Nothing was built and no test suite was run. The probes used the 0.2.0 release (commit `369e1dbb`), older than the code read (`872dc9507`); a fix made between them may be missing from the probes.
- No speed was measured except start-up and a tiny `check` (once, under load). The language server's latency, `fmt` on a project, `kavach test` on a large file, the index rebuild and the WebAssembly playground are open.
- VS Code was not started; extension gaps come from reading its code.
- Whether the server's unread stderr can actually fill its pipe in practice.
- How much debug information Cranelift can produce for an object file.
