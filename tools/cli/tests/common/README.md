# Shared test code

Code several test targets in `tools/cli/tests/` use, through `mod common;`. It is no test target of its own.

| File | What it holds |
|---|---|
| `mod.rs` | `Running`: names a program still running after a minute, so a run of many programs that hangs says which one |
