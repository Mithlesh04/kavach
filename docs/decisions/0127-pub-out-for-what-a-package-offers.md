# 0127. `pub(out)` for what a package offers to other packages

- **Status:** Accepted
- **Date:** 2026-11-17

## Context

Kavach warns about a name that is never used (1.4), so code being written still runs while mistakes show. A `pub` name was never warned, since another package might use it. But an application has no other packages using it, and in a library most `pub` names are shared only between the library's own modules. So a forgotten `pub fn` was never reported, in exactly the code where it is most common. A maintainer asked for a word that says "other packages use this", so that everything else can be checked.

## Decision

Three levels, each with its own warning (a maintainer confirmed this design on 2026-11-17):

```kavach
// in a library (src/lib.kav and its modules)
pub(out) fn parse(text: String) -> Doc { … }     // other packages too: never "unused"
pub(out) const version = "1.2"
pub(out) type Doc {
    pub title: String                            // fields: `pub` and private, as today
    body: [Node]

    pub fn word_count(self) -> Int { … }         // a `pub` method of a `pub(out)` type: offered
    fn reindex(self) { … }                       // this package only, as today
}
pub(out) use parser.{Token, tokenize}            // re-export

pub fn tokenize(text: String) -> [Token] { … }  // this package's modules: warned when unused
fn helper() { … }                                // this module: warned when unused
```

- **No marker:** this module only. Warned when unused, as today.
- **`pub`:** this package's other modules. A `pub` function, type or constant that nothing in the package uses gets a warning: "`tokenize` is `pub`, but nothing in this package uses it: remove it, or make it `pub(out)`".
- **`pub(out)`:** other packages too, and never warned as unused. It goes before `fn`, `type`, `const`, `interface`, `capability`, `agent` and `use`.
- The methods of a `pub(out)` type follow today's rule: its `pub` methods are offered, and the others stay in the package. Fields keep `pub` and private.
- **Only a library may write `pub(out)`.** In an application it is a compile error ("an application is used by no other package: write `pub`").
- **Editors show `pub(out)` in its own colour.** It gets its own scope in the VS Code grammar, a default colour the extension contributes, and an LSP semantic token, so the names a package offers stand out from plain `pub`.

## Consequences

- An application gets every unused-name warning, `pub` included.
- A library states its offered names in its code, and tools (docs, `kavach outline`, the LSP) can list them.
- The resolver's unused check replaces its blanket skip of `pub` (the fix for K46) with the three levels.
- The standard library and the packages (kavach-sql, kavach-postgres, Rath) mark the names they offer `pub(out)`.
- Spec 10.1 and 1.4, the parser, the resolver, the VS Code grammar and its test line, the LSP and the docs site change.

## Alternatives considered

- **`export`:** familiar from JavaScript and TypeScript, but a second word for visibility next to `pub`.
- **No keyword:** in a library, names reachable from `src/lib.kav` would be its offered names. That needs no new syntax, but nothing in the code would show which names are offered.
