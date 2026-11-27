# Shared test code

Code several test targets in `tools/cli/tests/` use, through `mod common;`. It is no test target of its own.

| File | What it holds |
|---|---|
| `programs.rs` | For the targets that run many programs natively (`native.rs`, `passes_off.rs`), through `#[path]`: finding the programs, their `// args:`, a panic's first line, and checking them several at once (`KAVACH_TEST_WORKERS`) |
| `mod.rs` | `interpret`: the interpreter's run of a test program, in a `kavach run` child when it starts itself again or reads standard input; `Running`: names a program still running after a minute, so a run of many programs that hangs says which one |
