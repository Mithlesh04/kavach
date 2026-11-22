# runtime/

Code that is linked into every Kavach program and runs alongside it. Written in Rust.

| Folder | Job |
|---|---|
| [core/](core/README.md) | Code shared by the interpreter and native code: `Decimal`, number printing (`kavach-runtime-core`) |
| [net/](net/README.md) | Networking for the interpreter and native code: TCP, UDP, TLS clients, the model call of `ask<T>` (`kavach-runtime-net`); apart from `core/` so programs without networking don't link TLS (decision 0024) |
| [gpu/](gpu/README.md) | GPU compute for `gpu fn`, and drawing into windows for `std.gpu`, through `wgpu` (`kavach-runtime-gpu`); apart from `native/` so programs without GPU code don't link it (decision 0044) |
| [window/](window/README.md) | Windows and their keyboard and mouse input for `std.window` through `winit` (`kavach-runtime-window`); apart from `native/` so programs without windows don't link it (decision 0104) |
| [input/](input/README.md) | Gamepads for `std.input` through SDL3: their state and gyroscopes, events to windows, rumble (`kavach-runtime-input`); apart from `native/` so programs without gamepads don't link it (decision 0124) |
| [audio/](audio/README.md) | Sound for `std.audio` through `cpal`: a mixer on the device's real-time thread, WAV and Ogg Vorbis (`kavach-runtime-audio`); apart from `native/` so programs without sound don't link it (decision 0114) |
| [watch/](watch/README.md) | Watching files and folders for `std.fs` through `notify` (`kavach-runtime-watch`); apart from `native/` so programs that watch nothing don't link it (decision 0116) |
| [image/](image/README.md) | Images for `std.image`: PNG and JPEG decoded to RGBA, PNG written (`kavach-runtime-image`); apart from `native/` so programs without images don't link it (decision 0117) |
| [font/](font/README.md) | Fonts for `std.font`: OpenType read and text shaped through `rustybuzz`, glyphs drawn into a distance atlas (`kavach-runtime-font`); apart from `native/` so programs without fonts don't link it (decision 0121) |
| [tracy/](tracy/README.md) | Spans and GPU work shown live in the Tracy profiler through `tracy-client` (`kavach-runtime-tracy`); linked only with `--tracy` (decision 0123) |
| [dialog/](dialog/README.md) | The system's dialogs for `std.app` through `rfd` (`kavach-runtime-dialog`); apart from `native/` so programs without dialogs don't link it (decision 0118) |
| [process/](process/README.md) | Other programs for `std.process`, run with no shell as trees that end together, through each system's own calls (`kavach-runtime-process`, decision 0105) |
| [matrix/](matrix/README.md) | The work of `std.matrix`: products in one fixed order, and fast ones through `gemm`, on flat lists of numbers (`kavach-runtime-matrix`, decision 0111) |
| [native/](native/README.md) | The run-time library of native programs: heap values, printing, panics; linked into executables (`kavach-runtime-native`) |
| [unpack/](unpack/README.md) | Packed executables: the format and the small C starter that unpacks a program built with `kavach build --compress` (`kavach-runtime-unpack`, decision 0081) |
| `memory/` | Empty: kept for allocation, arenas and cycle detection (debug) if they move out of `native/` |
| `tasks/` | Empty: the task scheduler lives in `core/` (`src/tasks.rs`), its native side in `native/` (`src/rt_tasks.rs`) |
| `agents/` | Empty: agents' guards (limits, approvals, the audit log) live in `core/` (`src/agents.rs`) and `native/` (`src/rt_agents.rs`) |
| `durable/` | Empty: the journal of durable handlers lives in `core/` (`src/journal.rs`) and `native/` (`src/rt_journal.rs`, `src/rt_replays.rs`) |

**Status:** every folder with a README is in use. Reference counting and the allocator live in `native/`; arenas are lowered by the compiler (`compiler/kir`). Tasks, agents and durable handlers were built in `core/`, so the interpreter and native code share them; the four empty folders are only places they could move to.
