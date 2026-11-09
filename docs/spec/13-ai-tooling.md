# 13. Tooling for AI: Fewer Tokens

AI assistants now write much of the world's code, and they pay for every token they read and write. Most of those tokens are spent on **finding and reading code**, not on writing the change. Kavach's goal: an AI assistant working on a Kavach codebase uses **at least 90% fewer tokens** than on the same codebase in another language, for the same task, with the same or better correctness.

This chapter defines the tools that make that possible. They work because Kavach code describes itself: a signature already says what a function takes, returns, can fail with, can touch, and promises (3.5). An assistant rarely needs the body.

## 13.1 Where tokens go today

| Step | What an assistant does today | Kavach tool | Saves |
|---|---|---|---|
| Find code | Text search, then read whole files to understand the hits | `kavach query` (13.2) | The reading |
| Understand code | Read the file and everything it imports | `kavach outline`, `kavach context` (13.3, 13.4) | Bodies it doesn't need |
| Change code | Rewrite or diff a file, repeating surrounding lines | `kavach edit` (13.5) | Repeated context |
| Fix errors | Read long messages, guess a fix, try again | Errors with fixes (13.6) | Retries |
| Test | Write test files for each scenario | Generated tests (chapter 11) | Test code |
| Learn the language | Read docs and examples | `llms.txt` (13.8) | Most of the docs |

## 13.2 Ask the compiler, don't read files: `kavach query`

The compiler knows exactly what every name refers to. Questions get exact answers, one line per hit, with no surrounding noise:

```
kavach query callers orders.total_paid      # who calls it
kavach query uses Payments                  # every function that can touch Payments
kavach query fails BankError                # every function that can fail with it
kavach query type Order                     # fields, variants, methods, invariants
kavach query find refund                    # names containing "refund"
```

```
src/billing.kav:14   pub fn invoice(orders: [Order]) -> Invoice ! BillingError
src/report.kav:8     fn daily_total(day: Date) -> Decimal uses Db
```

A function can fail with `BankError` when its signature says `! BankError`, or when it says `! Error` (any error, spec 5.5) and passes one on with `try`. `fails` lists both kinds.

A text search finds the word "refund" in comments, strings and unrelated names, and the assistant then reads files to sort them out. A query finds only the definitions and real references.

`kavach query tokens --format json <files or folders>` tells tools outside an editor (a docs site's code samples) where each name comes from, as the editor colors it. It prints one JSON line per file. Each token is `[line, column, length, type, [modifiers]]`: the line counts from 0, and the column and length count UTF-16 code units, as the editor protocol does. The types and modifiers are the language server's: a module brought in with `use` is a `namespace`, and what is used through it is `imported`; a built-in method is a `method` that is `defaultLibrary`. A file that doesn't compile gives its first error instead. A file inside a project brings the project's modules with it, as `kavach check` does.

```
kavach query tokens --format json src
```

```
{"file":"src/main.kav","tokens":[[3,10,6,"namespace",["imported"]],[3,17,5,"function",["imported"]]]}
{"file":"src/orders.kav","tokens":[[1,7,3,"method",["defaultLibrary"]]]}
```

## 13.3 Signatures are the summary: `kavach outline`

`kavach outline <file or module>` prints a module without function bodies: types, signatures, contracts, examples and doc comments.

```kavach
/// Sum of all paid orders.
pub fn total_paid(orders: [Order]) -> Decimal
    example total_paid([]) == 0
{ … }
```

Because Kavach signatures carry errors, capabilities and contracts, the outline is enough to call the code correctly. It is typically **5–15% of the file's tokens**.

## 13.4 Exactly the context needed: `kavach context`

```
kavach context orders.refund --budget 2000
```

Returns the full code of `orders.refund`, plus the outline (13.3) of everything it uses: the types it reads, the functions it calls, the capabilities it touches. It ranks what matters most and stops at the token budget. One call replaces the five to twenty file reads an assistant would otherwise make.

## 13.5 Edit by name, not by text: `kavach edit`

The assistant writes only the new code. It never repeats unchanged lines to place an edit:

```
kavach edit replace orders.total_paid < new_fn.kav    # replace one declaration
kavach edit add-field Order "discount: Decimal = 0"   # add a field
kavach rename Order.amount total                       # rename everywhere, safely
```

- An edit is checked before it is saved. If it breaks the build, nothing is written and the errors come back (13.6).
- `rename` and other refactors update every use across the workspace, using the resolver, not text replacement.

## 13.6 Errors that carry their fix

Every message has a short, machine-readable form:

```
kavach check --format json
{"file":"src/a.kav","line":3,"col":11,"code":"unknown-name","message":"`totl` is not defined","fix":{"replace":"totl","with":"total"}}
```

- One line per message, with no source excerpt unless asked for (`--context`).
- When the compiler knows the fix (a typo, a missing `-> Int`, `const` that should be `let`, a missing `try`), the message includes it as an edit. `kavach fix` applies every safe fix at once.
- The same messages are shown to people in the friendly form (with the caret under the problem).

## 13.7 Less to write

The language itself keeps code short without making it cryptic: inferred local types (2.10), no semicolons (1.1), named arguments with defaults (3.2), `.field` lambdas (3.3), `with` for updates (2.4), one-line contracts (7), and tests generated from them (11). The token benchmark (13.10) measures how many tokens the same program takes in Kavach and in other languages.

## 13.8 One guide and one server for every AI tool

- **`llms.txt`:** a compact guide to the whole language, generated from this spec, under 4,000 tokens. An assistant reads it once instead of the full spec.
- **`kavach mcp`:** a Model Context Protocol server exposing `query`, `outline`, `context`, `edit`, `check`, `test` and `run` as tools, so any AI assistant can use them directly. It runs locally; code never leaves the machine.

## 13.9 Always ready: the index

In a project (10.3), `kavach query` and `kavach context` keep an index of the program in `.kavach/index`, at the project's root. Later calls answer from it without checking the program again.

```
cd my-app
kavach query find refund          # checks the project once and writes .kavach/index
kavach query callers refund       # answers from the index
kavach context orders.refund      # so does this
```

- **No file needed.** Inside a project, `query` and `context` work on the whole project when no file is given, like `run` and `check` (10.5). Paths in answers are relative to the current folder.
- **What it holds:** each file of the program with a hash of its text; every declaration with its module, file, line, signature, outline (13.3) and code, the capabilities it uses and the error type it can fail with; and every reference from the program's code, with the declaration it names and the function it is in. Everything `query` and `context` answer comes from it.
- **Always correct:** before answering, a tool compares the hashes of the project's files (and of `kavach.toml`) with the index. If a file was added, removed or changed, it checks the program again and rewrites the index. A program with errors gets no index: the tool shows the errors instead.
- **Only a cache:** deleting `.kavach/index` changes no answer, only how fast it comes. Its format belongs to the compiler and may change with any version.
- **Names with a module:** `orders.refund` is `refund` in the module `orders` (10.1); `Order.total` is the method `total` of `Order`; `orders.Order.total` is both. A plain name that several modules declare is an error for `context`, which lists the full names to choose from.
- A single file outside a project has no index: it is checked on every call.

## 13.10 Measuring it

The claim must be measured, not assumed. `benchmarks/tokens/` holds realistic tasks done by an AI agent twice: once with plain file tools (read, search, write) and once with the Kavach tools.

| Task | Example |
|---|---|
| Find and change | Find every caller of a function and add a parameter |
| Extend | Add a field to a record and use it in two places |
| Fix | Fix a compile error in a large module |
| Feature | Add a small feature, with tests |

For each task the benchmark records input and output tokens, and whether the result is correct. The target is a **90% reduction on finding and editing tasks**. Results are published, including the tasks where the target is not reached.

## 13.11 When each part arrives

| Phase | Tools |
|---|---|
| 1 | `kavach outline`, `kavach query`, `--format json` messages (**done for single files**, [tools/ai](../../tools/ai/README.md)) |
| 2 | `kavach context`, `kavach mcp`, `llms.txt`, the index (**done**), fixes in messages, refreshing the index as files change |
| 3 | `kavach edit` and `rename`, the published token benchmark |

## Open questions

- Output format of `query`, `outline` and `context`: plain text (fewest tokens) or JSON (easiest to parse)? Possibly both, with text as the default.
- Which tokenizer to count with in the benchmark, since each AI model counts tokens differently.
- Should `context` also include the generated test cases of a function, so an assistant sees its behaviour without reading the body?
- Refreshing the index (13.9) as files change, from the language server or `kavach check --watch`, and only for the files that changed. Today one changed file means checking the whole program again on the next call.
