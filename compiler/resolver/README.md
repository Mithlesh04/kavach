# compiler/resolver (`kavach-resolver`)

Links every name in a file to its declaration, and checks the rules that only need names (spec 1.4, 1.5, 3.1):

- unknown names, with a suggestion: "`totl` is not defined; did you mean `total`?"
- a local used before its `const`
- a name declared twice in one scope
- assigning to a `const`, a parameter or `self`
- naming styles: `snake_case` values, `PascalCase` types (with the corrected spelling)
- a warning for names that mix look-alike scripts (Latin `a` and Cyrillic `а`)

Field and method names (`order.total`) need types, so the type checker resolves those. A bare variant found in several unions (`Open` in `Door` and `Shop`) is recorded in `ambiguous` for the type checker to settle from context (spec 2.6).

## Files

| File | Job |
|---|---|
| `src/lib.rs` | Public API: `resolve`, `resolve_modules`, `Resolution`, `Def`, `DefKind`, `Diagnostic` |
| `src/modules.rs` | Modules (spec 10.1–10.2): a scope per file, `use` between modules, `pub` checks; the built-in types of `std.*` modules (`Json`) visible everywhere |
| `src/resolver.rs` | Scopes, definitions, lookups, error messages and suggestions |
| `src/decls.rs` | Pass 1 collects top-level names (usable before they appear); pass 2 resolves each declaration |
| `src/walk.rs` | Statements, expressions, patterns, types; the assignment check |
| `src/naming.rs` | `snake_case`, `PascalCase` and mixed-script checks |
| `src/prelude.rs` | Built-in names: types, capabilities, prelude functions |
| `tests/names.rs` | What names refer to, and every error message |
| `tests/spec_sync.rs` | Every conformance test and example resolves as expected |

## Scopes

Prelude → module → function (generic parameters, parameters) → blocks. Special scopes: `result` in `ensure`; a record's fields in its `invariant`; the limited method's parameters plus `sum` and `count` in an agent's `limit` rules.
