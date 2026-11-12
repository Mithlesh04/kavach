# tests/conformance/

Small Kavach programs that each check **one rule** of the [language spec](../../docs/spec/README.md). They are written against the spec, not against any compiler, so any Kavach implementation can prove it is correct by passing them.

## Folders

Folders mirror the spec chapters. Each folder's README lists its tests and the spec section each one checks.

| Folder | Spec chapter |
|---|---|
| [01-basics/](01-basics/README.md) | [1. Basics](../../docs/spec/01-basics.md) |
| [02-types/](02-types/README.md) | [2. Types](../../docs/spec/02-types/README.md) |
| [03-functions/](03-functions/README.md) | [3. Functions](../../docs/spec/03-functions.md) |
| [04-control-flow/](04-control-flow/README.md) | [4. Control flow](../../docs/spec/04-control-flow.md) |
| [05-errors/](05-errors/README.md) | [5. Errors](../../docs/spec/05-errors.md) |
| [06-memory/](06-memory/README.md) | [6. Memory](../../docs/spec/06-memory.md) |
| [07-contracts/](07-contracts/README.md) | [7. Contracts](../../docs/spec/07-contracts.md) |
| [08-capabilities/](08-capabilities/README.md) | [8. Capabilities and agents](../../docs/spec/08-capabilities-and-agents/README.md) (8.1 so far) |
| [09-concurrency/](09-concurrency/README.md) | [9. Concurrency](../../docs/spec/09-concurrency.md) (9.1 so far) |
| [10-modules/](10-modules/README.md) | [10. Modules and packages](../../docs/spec/10-modules-and-packages/README.md) (10.1–10.3 so far) |
| [11-testing/](11-testing/README.md) | [11. Testing](../../docs/spec/11-testing.md) |
| [12-maintainability/](12-maintainability/README.md) | [12. Maintainability](../../docs/spec/12-maintainability.md) |
| [14-standard-library/](14-standard-library/README.md) | [14. Standard library](../../docs/spec/14-standard-library/README.md) |
| [15-gpu/](15-gpu/README.md) | [15. GPU](../../docs/spec/15-gpu.md) |

Chapter 13, and the rest of 8, 9 and 10, get tests when their features are scheduled (see the [roadmap](../../docs/vision/roadmap.md)).

## Test format

Every test is one `.kav` file named after the rule it checks (`const-cannot-be-reassigned.kav`). It starts with a header of comments:

```kavach
// Rule: A `const` binding cannot be reassigned.
// Spec: 1.4
// expect compile error

fn main() {
    const count = 1
    count = 2           // error: count is a let, so it cannot change
}
```

| Header line | Meaning |
|---|---|
| `// Rule: …` | The rule in one plain sentence. Required. |
| `// Spec: N.N` | The spec section that states the rule. Required. |
| `// expect output: …` | One line of standard output. Repeat it for each line, in order. Standard output must match these lines exactly. |
| `// expect exit: N` | The exit code. Default: `0` when the program runs. |
| `// expect compile error` | Compilation must fail. The line that must be reported ends with `// error: <why>`. The reason is for people; the runner checks only the line. |
| `// expect warning` | Compilation must succeed with a warning on each line that ends with `// warning: <why>`, and on no other line; then the program runs as usual. |
| `// expect panic` | The program must stop with a panic (5.6). |
| `// expect panic: <text>` | The panic message must contain `<text>`. Used only where the spec defines the message, e.g. contract failures (7.4). |
| `// expect stderr: …` | One line of standard error, like `expect output`; a place in the test's own file shows as its name alone (`name.kav:9`). Without it, standard error isn't checked. |
| `// expect no stderr` | Standard error must be empty. |
| `// expect test pass` / `// expect test fail` | Run with `kavach test` instead of `kavach run`. |
| `// args: …` | The program's arguments, separated by spaces (14.6). Default: none. |

## Rules for writing tests

- **One rule per file.** If a test checks two things, split it.
- **Only use what the spec defines.** If a test needs something the spec doesn't say (for example the wording of a compiler error), don't guess: add it to the chapter's **Open questions** and leave the test out.
- **Keep them tiny.** Most tests are under 20 lines.
- **Expected output follows the printing rules** in spec [1.11](../../docs/spec/01-basics.md#111-how-values-are-printed).
- When a spec rule changes, the test changes in the same commit (see [AGENTS.md](../../AGENTS.md)).

## Running

```
cargo test -p kavach-cli --test conformance
```

The runner ([tools/cli/tests/conformance.rs](../../tools/cli/tests/conformance.rs)) checks each file as its header says: compile errors must be found; programs are run (for now by the development interpreter, decision 0014) and their output, exit code and panic message compared. A failure shows exactly what differed.
