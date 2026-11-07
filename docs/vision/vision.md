# Vision

## The problem

Software is now written in a new way:

1. **AI writes much of the code; humans review it.** Reviewing 300 lines of generated code is slow, and people miss things.
2. **AI agents take real actions.** They issue refunds, change records, send emails and deploy code.
3. **Companies don't trust those agents in production.** Today's guardrails are scattered across prompts, glue code and permission checks added afterward. None of them is a hard guarantee.

Existing languages were designed for humans writing code by hand. Nothing in a Python or C# function tells you whether it can touch the network, spend money or leak a customer's card number.

## The answer

Kavach is a general-purpose language in which **what code is allowed to do is part of the code**, and is checked by the compiler and enforced by the runtime.

> **Easy to write. Fast to run. Safe for AI agents.**

## Who it is for

| Audience | Why they care |
|---|---|
| **Primary:** teams putting AI agents into production (fintech, e-commerce, SaaS, internal ops) | They can finally prove what an agent can and cannot do |
| **Secondary:** backend and systems developers | Native speed and memory safety in a language that is easy to learn |
| **Later:** user interfaces, GPU and embedded developers | A native UI toolkit, GPU compute and microcontroller builds come after the core platform ([platform](platform.md), Phase 3–4) |
| **Not now:** data science notebooks | Served through Python interop |

## Goals

| Goal | Target |
|---|---|
| Fast, measured | Native code, no GC. At least 80% faster than Java, C#, Go and Node.js on allocation-heavy work and startup; match Rust, C++ and Zig on CPU-bound work and beat their default I/O. Proven by published benchmarks ([decision 0017](../decisions/0017-native-platform-and-speed-targets.md)) |
| Easy to learn | A working developer is productive in **one day**. Small core, inferred types, errors that explain the fix |
| Built for AI and agents | Capabilities, limits, approvals, audit logs, typed AI calls and contracts are part of the language |
| Cheap for AI to work on | Goal: AI assistants use 90% fewer tokens on Kavach code, by querying the compiler instead of reading files ([spec 13](../spec/13-ai-tooling.md)) |
| Memory: automatic with freedom | Automatic by default with no GC pauses; arenas and `unsafe` for full control |
| Built to last 100 years | Written spec, stability promise, editions, conformance tests, open governance |
| Runs everywhere | Linux, Windows, macOS, WASM (web), Android (NDK), on x86-64, ARM and RISC-V |

## Non-goals

- **Being a better Python, Rust or Go for everything.** Speed and ease are table stakes, not the reason to switch.
- **A language for non-programmers.** End users will talk to AI in their own language; Kavach is the precise layer the AI writes and the system verifies.
- **Clever features.** If a feature makes code harder to read, it does not go in.

## Design principles (in priority order)

When two principles conflict, the higher one wins.

1. **Safe:** wrong programs should not compile; dangerous actions must be visible.
2. **Readable:** code is read far more often than it is written, by humans and AI alike.
3. **Simple:** one clear way to do each thing. Few concepts.
4. **Fast:** performance is the default, with escape hatches for the hottest code.
5. **Familiar:** borrow well-known syntax where it does not hurt the four above.

## How Kavach gets adopted

Adoption is **bottom-up**: developers try Kavach on their own, then bring it to their company. Kavach has to pass two tests.

**Test 1: win the developer in 30 minutes.**
- One-command install and a browser playground.
- A clear "wow" moment: an agent with guardrails that is provably safe, written in minutes.
- Works with their existing stack from day one (C, Python, JS interop; JSON; HTTP).
- AI assistants write correct Kavach (`llms.txt`, MCP server, small regular grammar).

**Test 2: help the developer convince their company.**

| Manager asks | Kavach answers with |
|---|---|
| Is it stable? | Semantic versioning, a stability promise, a public roadmap |
| Who maintains it? | Open governance, several maintainers, clear funding |
| Is it secure? | Security policy, published audits, `kavach audit` |
| Do we have to rewrite everything? | No. Adopt one module or one agent at a time |
| What if it fails? | Agents start in `observe` mode, which never blocks anything. Easy exit through interop |

**Trust ladder for agents:** `observe` (only report) → `suggest` (a person approves every action) → `enforce` (guardrails act automatically). A company never has to trust Kavach on day one.

**Adoption order:** our own production use → startups and independent developers → mid-size companies → enterprises.

## How we measure success

Not GitHub stars. These:

1. Agent actions running in production through Kavach.
2. Companies with Kavach in at least one production service.
3. Time for a new developer to ship their first working program.
4. Number of maintainers outside the founding team.
