# Kavach Language Specification

**Version:** 0.1 (draft) · **Edition:** 2026 (draft)

This specification is the source of truth for the language. If the compiler and the spec disagree, the compiler is wrong. If the spec is unclear, that is a bug in the spec.

## Chapters

| # | Chapter | Covers |
|---|---|---|
| 1 | [Basics](01-basics.md) | Files, comments, variables, literals, operators |
| 2 | [Types](02-types/README.md) | Built-in types, records, unions, optionals, interfaces, generics, `Json`, `Bytes` |
| 3 | [Functions](03-functions.md) | Functions, named arguments, lambdas, signatures |
| 4 | [Control flow](04-control-flow.md) | `if`, `match`, loops |
| 5 | [Errors](05-errors.md) | Errors as values, `try`, `catch`, `fail`, panics |
| 6 | [Memory](06-memory.md) | Automatic memory, shared state, arenas, `unsafe` |
| 7 | [Contracts](07-contracts.md) | `require`, `ensure`, `invariant` |
| 8 | [Capabilities and agents](08-capabilities-and-agents/README.md) | `uses`, `capability`, `agent`, limits, approvals, `ask<T>`, secrets |
| 9 | [Concurrency](09-concurrency.md) | Tasks, channels, shared state |
| 10 | [Modules and packages](10-modules-and-packages.md) | Modules, `use`, `pub`, project layout, tests, interop, editions |
| 11 | [Testing](11-testing.md) | Tests generated from types, contracts and `example`; capability fakes; agent exploration; crash fixes |
| 12 | [Maintainability at scale](12-maintainability.md) | File and function limits, workspaces, architecture rules, API checks, deprecation |
| 13 | [Tooling for AI: fewer tokens](13-ai-tooling.md) | `query`, `outline`, `context`, edit by name, errors with fixes, MCP server, token benchmark |
| 14 | [Standard library](14-standard-library/README.md) | Methods of built-in types, `std.fs`, `std.time`, `std.math`, `std.io`, `std.cli` |
| 15 | [GPU](15-gpu.md) | `gpu fn`, `map` on the GPU, same results as the CPU |

## Conventions in this spec

- Code blocks marked `kavach` are Kavach code.
- **Must** / **must not** are rules the compiler enforces.
- Each chapter ends with **Open questions**: things not decided yet.

## The language in one screen

```kavach
type Status = Pending | Paid | Refunded

type Order {
    id: Int
    amount: Decimal
    status: Status = Status.Pending
}

fn total_paid(orders: [Order]) -> Decimal {
    orders.filter(.status == Status.Paid).sum(.amount)
}

pub fn refund(order: Order) -> Order ! RefundError uses Payments
    require order.status == Status.Paid
    ensure result.status == Status.Refunded
{
    const _ = try Payments.refund(order.id, order.amount)    // the receipt isn't needed here
    order with { status: Status.Refunded }
}
```

- `type` declares records and unions. Types are values.
- Functions return their last expression.
- `! RefundError` lists the errors a function can return.
- `uses Payments` lists what the function is allowed to touch.
- `require` / `ensure` state intent and are checked.
- `kavach test` tests `refund` from its signature alone. No test file is needed.
