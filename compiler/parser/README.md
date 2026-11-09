# compiler/parser (`kavach-parser`)

Turns tokens into a syntax tree ([`kavach-ast`](../ast/README.md)). It never fails: it returns a tree for everything it could read plus every error, lexer errors included, so one run reports many problems.

```rust
let parsed = kavach_parser::parse("fn add(a: Int, b: Int) -> Int { a + b }");
assert!(parsed.errors.is_empty());
```

## Files

| File | Job |
|---|---|
| `src/lib.rs` | Public API: `parse`, `Parsed`, `SyntaxError` |
| `src/parser.rs` | Token cursor, error reporting, recovery |
| `src/items.rs` | Declarations: types, interfaces, `extend`, capabilities, agents (8.3), `use`, `test` |
| `src/fns.rs` | Functions: signature clauses in their fixed order (spec 3.5), `deprecated` (12.6) |
| `src/type_decls.rs` | `type` declarations: unions and records with fields, invariants, methods |
| `src/stmts.rs` | Blocks, `const`/`let`, assignment, `for`, `while`, `assert` |
| `src/exprs.rs` | Operator precedence (spec 1.8), `try`/`catch`/`fail`, calls, fields, `with` |
| `src/primary.rs` | Literals, strings, lambdas, lists and maps, `if`, `match` |
| `src/patterns.rs` | Patterns (spec 4.3) |
| `src/types.rs` | Types and generic parameters |
| `tests/expressions.rs`, `tests/declarations.rs` | Source → expected tree, printed as S-expressions |
| `tests/syntax_errors.rs` | Error messages, lines, and recovery |
| `tests/spec_sync.rs` | Every conformance test and example parses as expected; 3,000 random inputs never crash it |

## Design notes

- **Errors explain the fix**: "write the type of `x`: parameters always have a type", "`!` must come before `require`".
- **Recovery is line-based.** After an error the parser skips to the end of the line and carries on.
- **`<` after a name** starts type arguments only if they are followed by `(`: `ask<Decision>(…)` is a call, `a < b` a comparison.
- **Contextual keywords** (`fails`, `context`, the agent words `can`, `limit`, `approve`, `on`, `when`, `by`, `per`) are names to the lexer; the parser recognizes them only in their place (spec 1.6).
- **Variants start with a capital letter** (spec 1.5), so `Paid` in a pattern is a variant and `paid` a new binding.
