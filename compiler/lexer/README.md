# compiler/lexer (`kavach-lexer`)

Turns `.kav` source text into tokens: spec [chapter 1](../../docs/spec/01-basics.md). It never fails: it returns every token it could read plus a list of errors, so the compiler can report many problems in one run.

```rust
let lexed = kavach_lexer::lex("let total = 1 +\n    2");
// tokens: let total = 1 + 2 Eof   (the line break continues the statement)
```

## Files

| File | Job |
|---|---|
| `src/lib.rs` | Public API: `lex`, `Lexed`, `LexError` |
| `src/token.rs` | `Token`, `TokenKind`, `Keyword` (spec 1.6), punctuation table, line-continuation rule |
| `src/lexer.rs` | Main loop: spaces, comments, line breaks (spec 1.1), names, punctuation, brackets |
| `src/literals.rs` | Numbers, characters and escapes |
| `src/strings.rs` | Plain, raw and multi-line strings with `{…}` interpolation |
| `tests/tokens.rs` | Table-driven tests: source → expected tokens |
| `tests/spec_sync.rs` | Keywords match spec 1.6; every conformance test and example lexes as expected |
| `tests/random_input.rs` | 3,000 random inputs: no panics, valid spans, balanced strings |

## Design notes

- **Line breaks are tokens.** `Newline` ends a statement, except inside `( )` and `[ ]`, after an operator, or before a line starting with `.` (spec 1.1).
- **Strings are split** into `StrStart`, `Text`, `InterpStart` … `InterpEnd`, `StrEnd`, so interpolated expressions are ordinary tokens for the parser.
- **Number text is kept** for `Float` tokens, because context decides whether `0.1` is a `Float` or an exact `Decimal` (spec 1.7).
- **`>>` is one token.** The parser splits it when it closes two generic types (`Map<K, List<V>>`).
- **No dependencies.** Unicode names use the standard library; see `is_mark_like` in `src/lexer.rs`.

## Check

```
cargo test -p kavach-lexer
cargo clippy --all-targets
cargo fmt --check
```
