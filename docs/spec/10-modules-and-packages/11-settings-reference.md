# 10.11 Settings reference

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

| `[dependencies]` | a dependency's name | `{ path = "…" }`, or `{ git = "…" }` with at most one of `tag`, `branch` and `rev`, and `dir = "…"` for a folder of the repository; either may add `can = ["Net", …]` (the capabilities it may use) and `unsafe = true` | none | 10.12 |
| `[workspace]` | `members` | the folders of a workspace's packages, `*` matching any folder name: `["packages/*"]` | none | 10.14 |
| `[env]` | `file`, `vars` | the `.env` file to read; single variables that override it | `.env`, none | 10.13 |

Paths are relative to the folder `kavach.toml` is in. `[build.release]`, `[workspace]` and `[architecture]` are accepted and described where they are used (10.4), but not read yet.

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
| `KAVACH_UNPACKED` | a folder | Where a program built with `--compress` unpacks itself | 10.5 |
| `KAVACH_SERIAL` | any value | Loops that would run on all cores run on one | 9.3 |
| `KAVACH_ENV_FILE` | a file's path | The `.env` file a built program reads, instead of the one beside it | 10.13 |
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
