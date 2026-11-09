# 12. Maintainability at Scale

A language is easy at 500 lines. It must stay easy at 5 million lines, with many teams, many packages and many kinds of programs (services, CLIs, agents, libraries) in one codebase. Kavach makes the rules that keep big codebases healthy **part of the toolchain**, instead of leaving them to reviews and style guides.

All rules in this chapter are configured in `kavach.toml` and checked by the compiler.

## 12.1 File and function length

Small files are easier to read, review, test and give to an AI model. So Kavach limits file length by default:

```toml
[limits]
file_lines = 500          # default; counts every line, including comments and blank lines
fn_lines = 80             # default; lines in one function body
level = "error"           # "error" | "warn" | "off"
```

- **File limit:** a `.kav` file longer than `file_lines` is a compile error (or a warning with `level = "warn"`).
- **Function limit:** a function body longer than `fn_lines` is a **warning** by default. Long functions are sometimes clearer than split ones, so the developer decides.
- **Every line counts:** code, comments, doc comments and blank lines. A rule that skipped comments would push people to delete useful ones.
- Generated files (`kavach.lock`, `tests/found/`) and dependencies are never counted.

The error explains how to fix it, not only what is wrong:

```
error: billing.kav has 612 lines (limit 500)
  The file has 3 separate parts. Suggested split:
    type Invoice + 6 methods (lines 12–241)  →  billing/invoice.kav
    fn tax_for(...) + helpers (lines 250–398) →  billing/tax.kav
  The language server can do this: "Move to new file".
```

### Other files

Developers can apply the same limit to any text file in the project, such as docs or config:

```toml
[limits]
file_lines = 500
include = ["**/*.kav", "docs/**/*.md"]    # default: ["**/*.kav"]
exclude = ["src/generated/**"]
```

### Exceptions

An exception for one file is written in `kavach.toml`, not in the file itself, so all exceptions are in one place and visible in review:

```toml
[limits.files]
"src/tables/unicode.kav" = 3_000     # large data table
```

`kavach audit` lists every exception.

## 12.2 Workspaces: many packages, one repository

Large codebases are split into packages. A **workspace** builds and tests them together, with one lock file:

```toml
# kavach.toml at the repository root
[workspace]
members = ["apps/*", "libs/*", "agents/*"]
```

```
company/
├── kavach.toml           workspace settings, shared [limits] and [test]
├── kavach.lock           one lock file for everything
├── apps/
│   ├── shop-api/         a web service
│   └── admin-cli/        a command-line tool
├── libs/
│   ├── orders/           shared library
│   └── payments/
└── agents/
    └── refund-bot/       an AI agent
```

- Settings in the root (`[limits]`, `[test]`, `[architecture]`) apply to every member. A member can tighten them, but loosening them needs a line in the root file.
- Each member keeps its own `kavach.toml` with `[package]` and `[dependencies]`.
- Members depend on each other by path: `orders = { path = "../../libs/orders" }`.

## 12.3 Only rebuild and retest what changed

The compiler knows exactly which modules depend on which. So:

- `kavach build` recompiles only changed modules and the modules that use them.
- `kavach test --changed` runs only the tests (generated and hand-written) affected by the current changes.
- Results are cached in `.kavach/`. A CI server can share the cache between machines.

This keeps a large workspace fast: changing one library doesn't retest the whole company.

## 12.4 Architecture rules

In a big codebase, the worst problems are wrong dependencies: the UI calling the database directly, or a library depending on an app. Kavach lets a team **write the architecture down and have the compiler enforce it**:

```toml
[architecture]
layers = ["apps", "agents", "libs"]       # each layer may use only layers after it

[architecture.modules]
"libs/orders"   = { may_use = ["libs/payments"] }
"libs/payments" = { may_use = [] }
```

```
error: libs/payments/refund.kav:3 uses libs/orders
  libs/payments may not depend on libs/orders (kavach.toml, [architecture])
```

Always enforced, with no configuration:

- **No dependency cycles** between modules or packages.
- **Capabilities stay visible:** a module can only use a capability its package lists. This is the rule from chapter 8, checked across the whole workspace.

## 12.5 Stable public APIs

When many teams share a library, a careless change breaks people far away. Kavach checks this before a release:

```
kavach api check
```

It compares every `pub` signature with the last released version and reports changes that would break users:

| Change to a `pub` function | Breaking? |
|---|---|
| New optional parameter with a default | No |
| Removing a parameter, changing a type | Yes |
| New error variant in `! E` | Yes: callers' `match` may no longer cover every case |
| New capability in `uses` | Yes: callers must now grant it |
| Stronger `require` | Yes: some old calls become invalid |
| Stronger `ensure` | No |

A breaking change requires a new major version in `[package] version`. Because capabilities and contracts are in signatures, Kavach catches breaking changes that other languages miss.

## 12.6 Deprecation with automatic migration

An old function can point to its replacement. Tools then fix callers automatically:

```kavach
deprecated "use refund_order instead" fix refund_order(order)
pub fn refund(order: Order) -> Receipt ! RefundError uses Payments { … }
```

- `deprecated` goes on its own line before a function, after its doc comment and before `pub`.
- Calls to a deprecated function produce a warning that shows the message.
- `fix` is optional. Its expression is what each call becomes: the function's parameters in it stand for the call's arguments, and a parameter the call leaves out stands for its default. The names in it must exist, like any code.
- `kavach fix` rewrites calls using the `fix` expression, across the whole workspace, and the editor offers the same change as a quick fix. Where the `fix` expression uses a parameter inside an operator (`fix half_of(n * 2)`), an argument that is more than a name, a literal, a field or a call goes in parentheses, so it keeps its meaning: `half(a + b)` becomes `half_of((a + b) * 2)`.
- A call made through its module (`billing.refund(o)`) keeps the module on the names of the function's own module in the `fix`. A rewrite that would not compile where the call is (a replacement not imported there) is not made: the call keeps its warning.
- `kavach api check` refuses to remove a function in the same release that deprecates it.

```kavach
const receipt = try refund(order)        // warning: `refund` is deprecated: use refund_order instead
                                         // kavach fix: try refund_order(order)
```

Combined with editions (10.8), this lets a codebase evolve for years without manual mass edits.

## 12.7 One way to write it

Many small rules matter at scale because they remove arguments between teams:

- **One formatter, no options** (`kavach fmt`). Code from any team looks the same. It sets only the layout of lines, never what is on them:
  - 4 spaces per open `{`, `[` or `(`; several opened on one line count once;
  - one more level for a line that continues the one before (after an operator, a line starting with `.`, or a signature clause such as `require`), except inside `( … )` and `[ … ]`, which already indent it;
  - a comment line is indented like the code below it;
  - no trailing spaces, at most one blank line in a row, none just inside `{ … }`, and one newline at the end of the file;
  - multi-line strings are never changed.

  `kavach fmt --check` reports files that are not formatted, for CI.
- **One project layout** (10.3). Anyone can open any package and find their way.
- **Errors with fixes.** Every compiler error says what to change, and `kavach fix` applies the fix when it is safe: when it has one right answer, and adds no error. Today that is code that never runs taken out (4.7), a call to a deprecated function rewritten (12.6), `=` in a condition made `==` (4.1), `0..=xs.len()` made `0..xs.len()` (4.4), `x % 2 == 1` made `x % 2 != 0` (1.8), a changed `const` made `let`, and the `uses` a `pub` function needs added (8.1). The editor offers each as a quick fix, and also the fixes that may be wrong, which `kavach fix` leaves: a misspelled name changed to the closest one, and a `let` that hides an outer variable (1.4) made a change to that one.
- **No warnings in release builds** by default: `kavach build --release` treats warnings as errors unless `[build.release] warnings = "allow"`.

## Open questions

- Should `fn_lines` default to 80, or to `off` until people have used the language?
- Deprecating methods, types and fields, not only functions.
- Checking the type of a `fix` expression against the function's, at the declaration.
- Should `[architecture]` rules also apply between modules inside one package, or only between packages?
- Ownership: should `kavach.toml` map folders to teams (like a CODEOWNERS file), so tools know whom to ask?
