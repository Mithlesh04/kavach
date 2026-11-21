# Architecture Overview

How Kavach source code becomes a running program.

## Compiler pipeline

```
 .kav files
     │
     ▼
  lexer ──► parser ──► ast
                        │
                        ▼
                    resolver ......... names, modules, `use`
                        │
                        ▼
                    checker .......... types, capabilities, contracts,
                        │              secrets, ownership
                        ▼
                      kir ............ Kavach IR: reference counting
                        │              inserted and removed, optimizations
           ┌────────────┴────────────┐
           ▼                         ▼
   backend/cranelift          backend/llvm
   fast compile               fully optimized
   (kavach run --native)      (kavach build)
           │                         │
           └───────────┬─────────────┘
                       ▼
          native binary  +  runtime (linked in)
```

| Stage | Folder | Input → Output |
|---|---|---|
| Lexing | `compiler/lexer` | text → tokens |
| Parsing | `compiler/parser` | tokens → syntax tree (`compiler/ast`) |
| Resolving | `compiler/resolver` | syntax tree → every name linked to its definition |
| Checking | `compiler/checker` | resolved tree → typed tree, or clear errors |
| Lowering | `compiler/kir` | typed tree → Kavach IR (KIR) |
| Code generation | `compiler/backend/*` | KIR → machine code or WASM |
| Orchestration | `compiler/driver` | runs all stages; shared by the CLI and language server |

### What the checker verifies

The checker is where Kavach's guarantees come from:

1. **Types:** every value has a known type; no null.
2. **Capabilities:** a function only uses what its `uses` clause allows; an agent handler only uses what its `can` clause grants.
3. **Errors:** every possible error is handled or passed on with `try`.
4. **Secrets:** `secret` values never flow into prompts, logs or `print`.
5. **Ownership:** values used across tasks are safe to share; values inside an `arena` don't escape.
6. **Contracts:** proven where possible; otherwise turned into runtime checks.

## Runtime

Linked into every program.

| Part | Folder | Job |
|---|---|---|
| Memory | `runtime/memory` | Allocation, reference counts, arenas, cycle detection in debug builds |
| Tasks | `runtime/tasks` | Lightweight tasks, scheduler, non-blocking I/O |
| Agents | `runtime/agents` | Capability gate, limit counters, approval requests, audit log |
| Durable | `runtime/durable` | Checkpoints so handlers survive crashes and pause for approvals |

## How agent guardrails are enforced

Two layers. Neither one depends on the AI model behaving well.

| Guarantee | Compile time | Run time |
|---|---|---|
| Agent only calls capabilities in `can` | ✅ checker | ✅ capability gate (also checks generated plans) |
| Per-call limits (`amount <= 10_000`) | – | ✅ checked before every call |
| Windowed limits (`sum(amount) <= 50_000 per day`) | – | ✅ counters in the state store |
| Approvals (`approve … when … by …`) | – | ✅ handler pauses (durable) until a person decides |
| Secrets never reach prompts or logs | ✅ checker | ✅ automatic redaction in the audit log |
| Audit trail | – | ✅ every capability call recorded |

## Build modes

| Command | Backend | Contracts checked | Used for |
|---|---|---|---|
| `kavach run` | none: the interpreter (`compiler/interp`) | all | development, instant feedback |
| `kavach run --native` | Cranelift, compiled in memory | all | checking native code quickly |
| `kavach build` | LLVM where it supports the program so far, otherwise Cranelift ([decision 0018](../decisions/0018-llvm-backend-through-llvm-tools.md)); `KAVACH_BACKEND=cranelift` always uses Cranelift | all | executables |
| `kavach build --release` (not built yet) | LLVM | public functions (configurable, spec 7.4) | production |
| `kavach build --target wasm` (not built yet) | LLVM | configurable | browser and edge |

A program built with Cranelift runs on one core and as written: `spawn cores` tasks take turns on one core, and loop conversions and fused loops run only in LLVM builds.

## Tools on top of the compiler

```
tools/cli ──┐
tools/lsp ──┼──► compiler/driver
tools/ai  ──┘
```

The CLI, the language server and the AI tooling all use the same compiler driver, so error messages are identical everywhere.
