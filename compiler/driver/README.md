# compiler/driver (`kavach-driver`)

Runs the compiler pipeline on one file: lexer → parser → resolver → type checker. It collects every message and renders it for a terminal:

```text
error: `totl` is not defined; did you mean `total`?
 --> hello.kav:7:11
  |
7 |     print(totl)
  |           ^^^^
```

Everything that compiles Kavach goes through `compile()`, so messages look the same in the CLI, the interpreter and (later) the language server.

It also reads `kavach.toml` (spec 10.4), found in the file's folder or a parent: `[package] unsafe`, `[limits]` (spec 12.1) and `[test]` (spec 11.9). Other sections belong to later phases.

| File | Job |
|---|---|
| `src/lib.rs` | `compile`, `compile_sources`, `Options`, `Compiled` (tree, resolution, literal types, file table, messages), `render`; adds the standard library modules the program uses after its own files |
| `src/project.rs` | Projects: every file of `src/`, parsed as one source; `orders.total` rewritten to the name it links to |
| `src/std_modules.rs` | The standard library modules written in Kavach, and which of them a program uses: those it names with `std.<name>`, by a public type or variant, or through a built-in such as `ask` (decision 0024) |
| `src/config.rs` | `kavach.toml`: finding and reading it |
| `src/limits.rs` | File and function length limits, with a suggested split (spec 12.1) |
| `src/schema.rs` | The JSON Schema of a type: what `ask<T>` sends the model (spec 8.4) and `json.schema<T>()` gives (2.12), for the interpreter and native code |
