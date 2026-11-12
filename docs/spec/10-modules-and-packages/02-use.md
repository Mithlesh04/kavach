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
