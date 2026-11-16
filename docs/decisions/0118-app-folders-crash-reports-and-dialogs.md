# 0118. `std.app`: per-user folders, crash reports, and dialogs through `rfd`

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

A game or desktop tool needs a place for its saves and settings that is the user's own, a record of what happened when it crashed, and the system's dialogs to open a file or ask a question. Each system has its own rules: `%APPDATA%` on Windows, the XDG folders on Linux, `~/Library` on macOS, and its own dialogs. This is language request 9 (12) in the to-do. A maintainer chose it for this round (2026-11-16).

## Decision

- **`std.app`** holds the three.
- **Folders** by Kavach's own code, with no dependency: `app.data_dir(name)`, `app.config_dir(name)` and `app.cache_dir(name)` give the folder for the program called `name`, made if missing. They follow each system's rule: `%APPDATA%` and `%LOCALAPPDATA%` on Windows; `XDG_DATA_HOME`, `XDG_CONFIG_HOME` and `XDG_CACHE_HOME`, or their defaults under the home folder, on Linux; `~/Library/Application Support` and `~/Library/Caches` on macOS. They make a folder, so they need `Fs.write`.
- **Crash reports**: `app.crash_reports(folder)` makes every panic that ends the program also write a report there: the message, where it happened, the program, Kavach's version, the system and the time. It needs `Fs.write`.
- **Dialogs through `rfd`** (MIT): open a file, save a file, pick a folder, show a message, ask yes or no. On Linux `rfd` asks the desktop's portal, in pure Rust, so no GTK is needed. A dialog is a window the program shows, so it needs `Window`. It gives a path the person chose; reading it still needs `Fs.read`.
- Dialogs live in a crate of their own (`runtime/dialog`), so a program that shows none links none of `rfd` (decision 0024).

## Consequences

- Saves, settings, crash logs and file pickers work on every system with a few calls.
- Dialogs need a person at the screen, so tests check that they are typed and lowered, not that they show; generated tests skip them, as all `Window` functions.
- A crash report lists the functions that were running (added 2026-11-16, a maintainer's choice): `kavach build` writes a table of each function's start, name and place; at a panic the runtime walks the stack with the system's unwinder (`RtlCaptureStackBackTrace` and `RtlLookupFunctionEntry` on Windows, `backtrace` and `_Unwind_FindEnclosingFunction` elsewhere) and names the frames whose function starts it knows. It costs nothing until a panic and needs no new dependency. Frames show where each function is defined, not the line of the call: that needs line tables (debug info), which `kavach build` doesn't write. The interpreter notes each function a panic passes through, for the same list.

## Alternatives considered

- **The `dirs` crate for folders:** correct, but a dependency for a few environment variables. Rejected.
- **Native dialog code for each system:** three APIs to keep up. Rejected for now.
- **Dialogs only in packages:** every desktop tool would need one first. Rejected: opening a file is basic.
