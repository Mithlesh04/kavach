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

## Built, and points for a maintainer's review

Built on 2026-11-27 (spec 8.1 rule 7, 10.4, 10.11). Where this record didn't settle a point, the simplest option that fits it was taken. Each is **for a maintainer's review**:

1. **A default is counted where it is used.** What a function uses is worked out from its body and its `uses`, defaults included, so a function that doesn't print costs no `Console`: a contract, an agent and a package without the default can call it. Choice 3 read strictly (every exported signature carries every default) would make each `pub` function cost `Console` there. An interface's method, which has no body, costs its `uses` and its package's defaults.
2. **The standard library has no defaults:** its modules write every capability they use, so its own interfaces (`ai.Provider`, `test.Sample`) cost only what they list.
3. **A contract can't call an interface's method while its package has defaults** (7.5): any implementation may use them. With `default = ["Console"]`, no interface method of the package can be called in a contract. A way for an interface's method to say it uses nothing at all, as `uses ()` does for a function type (3.4), would lift this; it is not in the spec.
4. **Writing a default in `uses`** (`uses Console`) is allowed, with no warning.
5. **A function type that lists capabilities** (`fn() uses Net`) allows the defaults too; `uses ()` allows nothing (3.4). Comparing two function types (a value typed `fn() uses Console` where `fn() uses Net` is wanted) still compares only what they list.
6. **A default may be any built-in capability, or `Fs.read` or `Fs.write`.** A folder (`Fs("./data")`) can't be a default.
7. **Not built yet:** `kavach audit` doesn't list each package's defaults (choice 6).
