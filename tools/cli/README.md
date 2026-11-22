# tools/cli (`kavach`)

The `kavach` command.

```
kavach new <name>                     create a project (spec 10.5)
kavach run <file.kav> [-- args…]      check a file, then run it
kavach run --native <file.kav>        compile it to machine code first
kavach build <file.kav> [-o <out>]    write an executable
kavach test <file.kav> [--seed N] [--deep]   examples, test blocks and generated tests
kavach check <file.kav>               only check it for errors
kavach check --format json <file.kav>
kavach outline <file.kav>
kavach query callers|uses|fails|type|find <name> [file.kav]
kavach context <name> [file.kav] [--budget N]
kavach mcp
kavach lsp                            language server for editors (tools/lsp)
kavach --version
```

Inside a project (a folder with `kavach.toml`, or below it), `run`, `build`, `test` and `check` need no file: they use `src/main.kav`, and `build` writes `.kavach/bin/<name>`. So do `query` and `context`, which answer from the project's index in `.kavach/index` (spec 13.9).

Install it from the repository root with `cargo install --path tools/cli`, or run it without installing: `cargo run -p kavach-cli -- run file.kav`.

Exit codes: 0 success, 1 compile errors or an error returned from `main`, 101 a panic (spec 5.8), 2 wrong usage, 3 not supported by the native compiler yet.

| File | Job |
|---|---|
| `src/main.rs` | Commands |
| `src/help.rs` | The command list, and each command's own lines for `kavach <command> --help` (spec 10.5) |
| `src/watch.rs` | `kavach run --watch`: the program run again on each change of the project's files; and `kavach run --hot`, which has the interpreter swap in changed functions, running again only when more than bodies changed |
| `src/native.rs` | `kavach build` and `kavach run --native`: native code with LLVM or Cranelift |
| `src/context.rs` | `kavach query` and `kavach context`, from the program's index |
| `src/project.rs` | `kavach new`, and finding the project's entry file |
| `src/deps.rs` | `kavach add` and `kavach update`, and fetching dependencies before a compile (spec 10.12) |
| `src/library.rs` | `kavach build --lib`: a shared library and its C header (spec 10.10) |
| `src/fmt.rs` | `kavach fmt`: names first, then the layout of each file |
| `src/keep.rs` | `kavach test --keep`: passing found cases become `example` clauses (spec 11.4) |
| `tests/conformance.rs` | The conformance runner: every test in `tests/conformance/`, plus examples 01–04 |
| `tests/native.rs` | Native code must match the interpreter on every program it supports |
| `tests/project.rs` | `kavach new` and commands without a file inside a project |
| `tests/packages.rs` | A git dependency: `kavach add`, the commit `kavach.lock` keeps until `kavach update`, and a package using `unsafe` refused until allowed (spec 10.12) |
| `tests/context.rs` | `query` and `context` without a file inside a project |
| `tests/keep.rs` | `kavach test --keep` |
| `tests/stdlib.rs` | The standard library modules written in Kavach pass `kavach test` |
| `tests/tls.rs` | TLS servers and mutual TLS, with certificates made for the test by `openssl` |
| [`tests/approvals/`](tests/approvals/README.md) | Approvals that wait for a person, across restarts, and answered in code (spec 8.3) |
| [`tests/common/`](tests/common/README.md) | Code the test targets share: a watch that names a program still running after a minute |
| `tests/ai_crowd.rs` | Forty `ask`s at once against a fake provider that takes four at a time: all get through their provider's gate (spec 8.4) |
| `tests/ai_providers.rs` | Each kind of trouble as its own `AiError`, `retry-after` honoured, and a program's own provider, against a fake server (spec 8.4, decision 0140) |
| `tests/fake_gpus.rs` | `system.gpus()` with `KAVACH_FAKE_GPUS`: AMD, NVIDIA and Intel GPUs on any machine, best kind first, interpreted and natively (spec 14.25) |
| `tests/gpu_choice.rs` | Choosing a GPU (spec 15.6): `gpu.devices()`, a missing GPU stopping the start, `gpu.on`, `on:`, `on gpu(n)` and `gpu(n) fn` with fake AMD and NVIDIA GPUs and Windows' WARP as a real second device, and each real GPU by number, interpreted and natively |
| `tests/gpu_split.rs` | One `map` split across GPUs (spec 15.6, decision 0144): the CPU's values in order, exact sums, records, several passes a part, `check`, the first part's panic, a chosen device never split, with two fake devices opening this machine's real GPUs |
| `tests/gpu_passes.rs` | A `map` too big for one pass of the GPU (spec 15.2): records kept in a local after several passes, made small by `KAVACH_GPU_WORDS`, give the CPU's results, natively |
| `tests/limits_parallel.rs` | A limit's total holds when 30 handlers pay at once: exactly 10 refunds of 100 under `sum(amount) <= 1000 per day` |
| `tests/version.rs` | `kavach --version`, `kavach version <program>`, and `--version` answered by a program only with `[build] version_flag` (spec 10.5) |
| `tests/token_budget.rs` | Ten `ask`s at once under `tokens <= 200 per day`: each `asks_under_way` setting lets through what it promises (spec 8.3) |
| `tests/stopping.rs` | A program holding its stop finishes its work after `SIGTERM`, and a server answers the request under way (decision 0088; Unix) |
| `tests/workspaces.rs` | A workspace: one lock and `kavach_modules/` at the root, members using each other by path, `check` and `build` per member; a git dependency's `dir`, refused when it leaves the repository (spec 10.14) |
| `tests/watch.rs` | `kavach run --watch` runs the program again when its file changes (spec 10.5) |
| `tests/windows_resources.rs` | `kavach build` gives a Windows executable a manifest and the icon `[build] icon` names; an icon that isn't a PNG is refused (spec 10.5) |
| `tests/fonts.rs` | A font file read, text shaped into placed glyphs and drawn into a distance atlas, interpreted and natively (spec 14.31) |
| `tests/c_libraries.rs` | A library built with `kavach build --lib` loaded by another program while running (`extern c library`), in memory and built with each backend; on Linux, records by value to a C library compiled with `cc` (spec 10.7) |
| `tests/hot.rs` | `kavach run --hot`: a function changed while the program runs is swapped in keeping its values, and a new type runs it again (spec 10.5) |
| `tests/online.rs` | Tests that need the internet (`https://`); they run only with `--ignored` |
