# Examples

Read them in order. Each one introduces a few ideas.

| # | Example | Shows |
|---|---|---|
| 01 | [hello](01-hello/hello.kav) | A one-line script |
| 02 | [basics](02-basics/basics.kav) | Records, unions, functions, lists, `match`, the dot shorthand |
| 03 | [errors](03-errors/errors.kav) | `! Error` types, `fail`, `try`, `catch`, matching on results |
| 04 | [contracts](04-contracts/bank.kav) | `require`, `ensure`, `invariant`, `example`; tested with no test file |
| 05 | [refund-bot](05-refund-bot/) | A full project: capabilities, an agent with limits and approvals, `ask<T>`, secrets |

Examples 01–04 run today with the development interpreter; 05 needs agents (Phase 2). If anything here is confusing, that is a bug in the design.

From the repository root (or use `kavach` directly after `cargo install --path tools/cli`):

```
cargo run -p kavach-cli -- run examples/02-basics/basics.kav
cargo run -p kavach-cli -- test examples/04-contracts/bank.kav
```

The commands, once installed:

```
kavach run 01-hello/hello.kav
kavach test 04-contracts/bank.kav
cd 05-refund-bot && kavach run --dry-run
```
