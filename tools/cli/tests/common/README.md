# Shared test code

Code several test targets in `tools/cli/tests/` use, through `mod common;`. It is no test target of its own.

| File | What it holds |
|---|---|
| `mod.rs` | `interpret`: the interpreter's run of a test program, in a `kavach run` child when it starts itself again or reads standard input; `Running`: names a program still running after a minute, so a run of many programs that hangs says which one |
