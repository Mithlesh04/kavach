# runtime/

Code that is linked into every Kavach program and runs alongside it. Written in Rust.

| Folder | Job |
|---|---|
| [core/](core/README.md) | Code shared by the interpreter and native code: `Decimal`, number printing (`kavach-runtime-core`) |
| [net/](net/README.md) | Networking for the interpreter and native code: TCP, UDP, TLS clients, the model call of `ask<T>` (`kavach-runtime-net`); apart from `core/` so programs without networking don't link TLS (decision 0024) |
| [gpu/](gpu/README.md) | GPU compute for `gpu fn`, and drawing into windows for `std.gpu`, through `wgpu` (`kavach-runtime-gpu`); apart from `native/` so programs without GPU code don't link it (decision 0044) |
| [window/](window/README.md) | Windows and their keyboard and mouse input for `std.window` through `winit` (`kavach-runtime-window`); apart from `native/` so programs without windows don't link it (decision 0104) |
| [process/](process/README.md) | Other programs for `std.process`, run with no shell as trees that end together, through each system's own calls (`kavach-runtime-process`, decision 0105) |
| [native/](native/README.md) | The run-time library of native programs: heap values, printing, panics; linked into executables (`kavach-runtime-native`) |
| [unpack/](unpack/README.md) | Packed executables: the format and the small C starter that unpacks a program built with `kavach build --compress` (`kavach-runtime-unpack`, decision 0081) |
| `memory/` | Allocation, reference counting, arenas, cycle detection (debug) |
| `tasks/` | Lightweight task scheduler and non-blocking I/O |
| `agents/` | Enforces capabilities, limits and approvals at runtime; writes the audit log |
| `durable/` | Checkpoints so agent handlers survive crashes and can pause for approvals |

**Status:** `core/`, `net/` and `native/` are in use. `native/` holds reference counting for now; it moves into `memory/` when arenas and cycle detection arrive. `memory/` and `tasks/` start in Phase 1; `agents/` and `durable/` in Phase 2.
