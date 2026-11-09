# compiler/ast (`kavach-ast`)

The syntax tree: what the parser produces and every later stage reads. It mirrors the source closely. Names are not resolved and types are not checked yet; that is the job of `resolver/` and `checker/`.

| File | Contents |
|---|---|
| `src/lib.rs` | `Ident`, `Path`, and the `Span` used for error positions |
| `src/items.rs` | `File`, `Item` and declarations: functions with their full signature (spec 3.5), types, interfaces, `extend`, capabilities, agents, `use`, tests |
| `src/exprs.rs` | Statements, expressions and patterns |
| `src/types.rs` | Types as written: `Int`, `[T]`, `[K: V]`, `T?`, `(A, B)`, `fn(A) -> B` |
| `src/visit.rs` | `VisitMut`: a walk over the whole tree, for passes that change it |

Every node keeps its `Span`, so errors in any stage can point at the exact source text. Nodes that had a syntax error become `ExprKind::Error` or `TypeKind::Error`, so later stages can skip them without crashing.
