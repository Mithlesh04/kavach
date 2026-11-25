# Kavach

> **कवच (Kavach)** means *armor*.
> A programming language that is **easy to write, fast to run, and safe for AI agents.**

**Status:** pre-alpha. Phase 1 done; Phase 2 under way ([roadmap](docs/vision/roadmap.md)). Programs can be checked and run with `kavach run` (interpreted), and compiled to native code with `kavach run --native` and `kavach build` for a growing subset. The [language spec](docs/spec/README.md) is the source of truth.

Website: [kavachlang.com](https://kavachlang.com/)

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
        const decision = try ask<RefundDecision>(`Should we refund order {order.id}? Note: {order.note}`)
        if decision.approve {
            const receipt = try Payments.refund(order.id, order.amount)
            try Email.send(order.customer_email, `Refund {receipt.id} is on its way.`)
        }
    }
}
```

The agent cannot call anything outside `can`. Refunds above 10,000 are blocked, refunds above 5,000 wait for a person from finance, and every call is written to an audit log.

## Install

Kavach is pre-alpha. Ready-built releases are on the [releases page](https://github.com/Mithlesh04/kavach/releases), for Windows and Linux on x86-64. Each archive has a `.sha256` file to check it against.

- **Windows:** download and run [`scripts/install-kavach.ps1`](scripts/install-kavach.ps1). It installs the newest release.
- **Linux:** download and run [`scripts/install-kavach.sh`](scripts/install-kavach.sh). It installs the newest release into `~/.kavach`, links `kavach` into `~/.local/bin`, and checks for a C compiler, which linking needs. `kavach` needs ALSA's sound library, which desktop Linux has; on a server, install `libasound2t64` (Ubuntu 24.04, Debian 13), `libasound2` (older Ubuntu and Debian) or `alsa-lib` (Fedora).
- **Try it without installing:** the [playground](https://kavachlang.com/playground/) checks Kavach code in the browser.

### macOS

There is no ready-built macOS release yet: we have no Mac to test one on. You can build Kavach from source instead:

1. Install the Xcode command-line tools, for the linker: `xcode-select --install`. Then install CMake, which building the gamepad library needs: `brew install cmake`.
2. Install Rust with [rustup](https://rustup.rs), then add LLVM's tools, which `kavach build` uses: `rustup component add llvm-tools`.
3. Clone this repository and build: `cargo build --release -p kavach-cli -p kavach-runtime-native`.
4. Run `target/release/kavach`, for example `target/release/kavach run hello.kav`.

Without LLVM's tools, `kavach build` still works through its other backend, Cranelift, but the programs it makes are slower. Windows and pictures on screen (`std.window`, `std.gpu` drawing) aren't available on macOS yet. Nothing on macOS is tested yet, so please [tell us](https://github.com/Mithlesh04/kavach/discussions) what works and what doesn't.

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
| [website/](website/README.md) | The official docs site, [kavachlang.com](https://kavachlang.com/) ([decision 0033](docs/decisions/0033-docs-site-in-the-repo.md)) |
| [examples/](examples/README.md) | Example programs, in learning order |
| [tests/](tests/README.md) | Language-level tests (conformance and integration) |
| [benchmarks/](benchmarks/README.md) | Performance comparisons against other languages |
| [scripts/](scripts/README.md) | Helper scripts for developing Kavach itself |

## Start reading

1. [Vision](docs/vision/vision.md): the problem, who Kavach is for, and how it gets adopted
2. [Roadmap](docs/vision/roadmap.md): what gets built, in what order
3. [Language spec](docs/spec/README.md): how the language works
4. [Examples](examples/README.md): what Kavach code looks like

## Feedback and contributing

The most useful thing you can do is tell us where the design is wrong:

- **[Discussions](https://github.com/Mithlesh04/kavach/discussions)** — the language's open design questions live there; your opinion directly shapes the spec.
- **[Good first issues](https://github.com/Mithlesh04/kavach/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)** — small, self-contained ways in, from porting a benchmark to reporting your first 15 minutes.
- **[Playground](https://kavachlang.com/playground/)** — the real compiler checks your program in the browser, nothing to install.

For code changes, see [CONTRIBUTING.md](CONTRIBUTING.md). Rules for every contributor, including AI assistants and agents, are in [AGENTS.md](AGENTS.md).

## License

Kavach is free to use under either of these two licenses. You choose which one:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

[Decision 0011](docs/decisions/0011-license-apache-or-mit.md) explains why there are two.
