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
   (kavach run, debug)        (kavach build --release, WASM)
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
| `kavach run` | Cranelift | all | development, instant feedback |
| `kavach build` | LLVM where it supports the program so far, otherwise Cranelift ([decision 0018](../decisions/0018-llvm-backend-through-llvm-tools.md)) | all | executables |
| `kavach build --release` | LLVM | public functions (configurable) | production |
| `kavach build --target wasm` | LLVM | configurable | browser and edge |

## Tools on top of the compiler

```
tools/cli ──┐
tools/lsp ──┼──► compiler/driver
tools/ai  ──┘
```

The CLI, the language server and the AI tooling all use the same compiler driver, so error messages are identical everywhere.
