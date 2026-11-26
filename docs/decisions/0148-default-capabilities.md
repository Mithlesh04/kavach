# 0148. Capabilities every function has by default, set in kavach.toml

- **Status:** Accepted (details for a maintainer's review)
- **Date:** 2026-11-26

## Context

Printing needs `Console` (spec 8.1). `pub` functions must write their `uses`, and since [decision 0147](0147-interfaces-declare-uses.md) interface methods must too. Printing is used almost everywhere, so `uses Console` appears on most signatures and says little. Yet some code must not print: a library whose output would corrupt a protocol on standard output (a language server, an MCP server, a program writing JSON to a pipe), a module meant to stay pure, an agent.

A maintainer asked (2026-11-26) for printing allowed by default, with a flag in `kavach.toml` listing the default capabilities: removing one takes it away, and it may list several.

## Decision

```toml
[capabilities]
default = ["Console"]   # what every function may use without writing `uses`
```

1. **`[capabilities] default`** lists the capabilities every function in the package has without writing `uses`. Left out, it is `["Console"]`. `default = []` makes every effect explicit again.
2. **Any built-in capability may be listed**, several at once (`["Console", "Clock"]`), except `System.critical`, `Approvals` and `Foreign`, which are always written by name (a compile error names them).
3. **A default capability is part of every signature.** A `pub` function or interface method that leaves it out still has it, and the compiler writes it into the signature other packages see. A package whose own `default` doesn't list it must write `uses` to call such a function, so taking a capability away also takes it from what the package calls.
4. **Agents are not covered.** An agent's handlers may use only what its `can` lists (8.3), so an agent prints only when `can` lists `Console`.
5. **A dependency's defaults are its own**, from its `kavach.toml`. The program can still refuse it what it would get: grants per dependency (10.12) may leave out a default capability, and a call into the dependency that needs it is then a compile error.
6. **`kavach audit`** lists, per package, the default capabilities and the functions that use them.

## Consequences

- Printing works out of the box; signatures show the effects that matter.
- "No `uses`" means "uses only the package's defaults". With the default left as it is, a function without `uses` may print; it may still touch no file, network or other outside resource.
- The checker adds the package's defaults to every function's allowed set and to exported signatures; `kavach.toml` gains the table (spec 10.4, 10.11).

## Alternatives considered

- **Opting out in code** (`pure fn`): one keyword per function or module. Rejected in favour of one place, the package's settings, which the maintainer asked for.
- **Keeping `Console` explicit and inferring more:** the signature stays complete, but every printing `pub` function and interface method still carries it.
- **No change.**
