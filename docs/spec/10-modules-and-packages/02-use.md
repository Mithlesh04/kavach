# 10.2 use

```kavach
use orders                          // then write orders.total(...)
use billing.invoice                 // then write invoice.create(...)
use billing.invoice.{Invoice, create}   // bring names in directly
use std.json                        // standard library
use stripe                          // a dependency, by package name
use std.collections.Set as HashSet  // rename
```

Resolution order: local modules (from `src/`), then `std`, then dependencies. A local module with the same name as a dependency is a compile error. Rename one of them.

The **prelude** is always available without `use`: built-in types, `print`, `debug`, `ask`, `panic`, `env`, `Shared`, `Channel`.

**Re-exports.** `pub use` in a module makes the names it brings in part of that module too, so a package can declare each public type in a file of its own and show them all from its root ([decision 0101](../../decisions/0101-re-exports.md)):

```kavach
// src/lib.kav of the package `pg`
pub use client.{Client, connect}
pub use pool.{Pool}
```

```kavach
// a program using `pg`
use pg

const c = pg.connect(url)        // the `connect` declared in pg's client.kav
const p: pg.Pool = pg.Pool(…)
```

- A re-exported name is the same declaration, not a copy: `pg.Client` and `pg.client.Client` are one type.
- Only names are re-exported, and each must be `pub` where it is declared. `pub use orders`, a whole module, is a compile error.
- A re-exported name can be re-exported again.

