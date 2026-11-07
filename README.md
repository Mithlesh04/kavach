# Kavach

> **कवच (Kavach)** means *armor*.
> A programming language that is **easy to write, fast to run, and safe for AI agents.**

**Status:** pre-alpha, Phase 1. Programs using spec chapters 1–7 can be checked and run with `kavach run` (interpreted), and compiled to native code with `kavach run --native` and `kavach build` for a growing subset. The [language spec](docs/spec/README.md) is the source of truth.

Website: [mithlesh04.github.io/kavach](https://mithlesh04.github.io/kavach/)

---

## Why Kavach

AI now writes a large share of our code, and AI agents now take real actions: they move money, change data and send messages. Existing languages were designed for people writing code by hand. None of them tells you at a glance what a piece of code is **allowed** to do.

Kavach makes that visible and enforced:

- **Easy:** small language, clean syntax, inferred types, errors that explain the fix.
- **Fast:** compiled to native code. No JIT warm-up and no garbage-collector pauses.
- **Safe memory, with freedom:** automatic by default, with arenas and `unsafe` when you need control.
- **Capabilities in the signature:** every function and agent declares what it can touch (network, files, AI, payments…), and the compiler checks it.
- **Agents with guardrails:** limits, human approvals and audit logs are part of the language.
- **Contracts:** `require` / `ensure` state intent in one line, so humans review intent instead of hundreds of lines.
- **Tests without test files:** `kavach test` generates test data from types and contracts, fakes every outside system, and suggests fixes for every crash it finds.
- **Cheap for AI to work on:** AI assistants ask the compiler (`kavach query`, `outline`, `context`) instead of reading whole files. Goal: 90% fewer tokens for the same task. Connect any assistant with `kavach mcp`; [llms.txt](llms.txt) is the language in about 1,350 tokens.
- **Built for big codebases:** file length limits, enforced architecture rules and API break checks are part of the toolchain.

## A quick look

```kavach
agent RefundBot {
    can Ai, Payments.refund, Email.send

    limit Payments.refund {
        amount <= 10_000
        sum(amount) <= 50_000 per day
    }

    approve Payments.refund when amount > 5_000 by "finance"

    on request(order: Order) ! Error {
        const decision = try ask<RefundDecision>("Should we refund order {order.id}? Note: {order.note}")
        if decision.approve {
            const receipt = try Payments.refund(order.id, order.amount)
            try Email.send(order.customer_email, "Refund {receipt.id} is on its way.")
        }
    }
}
```

The agent cannot call anything outside `can`. Refunds above 10,000 are blocked, refunds above 5,000 wait for a person from finance, and every call is written to an audit log.

## Repository map

| Folder | What lives there |
|---|---|
| [docs/](docs/README.md) | Vision, roadmap, language spec, architecture, design decisions |
| [compiler/](compiler/README.md) | Turns `.kav` source into native code or WASM |
| [runtime/](runtime/README.md) | Memory, task scheduler, agent guardrails, durable execution |
| [stdlib/](stdlib/README.md) | Standard library, written in Kavach |
| [tools/](tools/README.md) | `kavach` CLI, formatter, language server, package manager, AI tooling |
| [interop/](interop/README.md) | Calling C, Python and JavaScript from Kavach |
| [editors/](editors/README.md) | Editor support: the VS Code extension ([how to install it](editors/vscode/README.md#install)) |
| [website/](website/README.md) | The official docs site, [mithlesh04.github.io/kavach](https://mithlesh04.github.io/kavach/) ([decision 0033](docs/decisions/0033-docs-site-in-the-repo.md)) |
| [examples/](examples/README.md) | Example programs, in learning order |
| [tests/](tests/README.md) | Language-level tests (conformance and integration) |
| [benchmarks/](benchmarks/README.md) | Performance comparisons against other languages |
| [scripts/](scripts/README.md) | Helper scripts for developing Kavach itself |

## Start reading

1. [Vision](docs/vision/vision.md): the problem, who Kavach is for, and how it gets adopted
2. [Roadmap](docs/vision/roadmap.md): what gets built, in what order
3. [Language spec](docs/spec/README.md): how the language works
4. [Examples](examples/README.md): what Kavach code looks like

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Rules for every contributor, including AI assistants and agents, are in [AGENTS.md](AGENTS.md). While Kavach is this early, the most useful contributions are feedback on the spec and real use cases.

## License

Kavach is free to use under either of these two licenses. You choose which one:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

[Decision 0011](docs/decisions/0011-license-apache-or-mit.md) explains why there are two.
