# 0066. The clipboard through `arboard`

- **Status:** Accepted (2026-11-04)
- **Date:** 2026-11-04
- **Changes:** spec 8.1, 14.17; `runtime/core` gains a dependency

## Context

The to-do's item 6 (Mithlesh, 2026-11-04) asks for modules for what is connected to the machine, each behind a capability. The clipboard is one: reading it is private data (Mithlesh's words), and every device needs a capability. Each system has its own clipboard interface: Windows' clipboard functions, macOS's pasteboard, and on Linux X11 selections or Wayland's data-control protocol.

## Decision

- **The clipboard comes through the `arboard` crate**, the one the Rust ecosystem uses for this (it is 1Password's, and used by many editors and terminals). It is built without its image support (`default-features = false`), so it pulls in no image crates.
- **One capability, `Clipboard`, for reading and writing.** Writing replaces what a person copied, so it needs permission too.
- **Text only, for now** (`read_text`, `write_text`). Images wait for a typed image value, which `std.camera` needs as well.
- In native code it is a runtime entry of its own (decision 0024), so only a program that uses the clipboard links `arboard`.
- Under `kavach test` the clipboard is a fake in memory (spec 11.5), as the files and the clock are.

## Consequences

- On Linux with X11, what a program writes is kept only while it runs, unless a clipboard manager takes it (spec 14.17). That is how X11 works, and other languages' clipboard libraries have the same limit.
- A machine with no clipboard (a server, WSL without a desktop) gives `Unavailable(reason)`, not a crash.

## Alternatives considered

- **Each system's interface written in the runtime.** That is more code to keep (three systems, and two on Linux) for what `arboard` already does.
- **`copypasta`.** It is older, has fewer users, and its Linux support is less complete.
