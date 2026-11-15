# 0116. Watching files through `notify`

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

A game reloads its shaders, levels and code when they change on disk. Tools rebuild when a source file is saved. Both need to hear about changes, not read every file again and again. Each system tells programs in its own way: `ReadDirectoryChangesW` on Windows, inotify on Linux, FSEvents on macOS. This is language request 9 (8) in the to-do. A maintainer chose it for this round (2026-11-16).

## Decision

- **`notify`** (CC0-1.0, no conditions) watches files and folders on Windows, Linux and macOS.
- `fs.watch(path)` gives a `Watcher` of a file, or of a folder and everything in it. A program reads its changes two ways, as it reads a window's events (decision 0104): `w.next()` waits for one, and `w.poll()` gives those that came, for a game's frame loop.
- A change is a path and a kind: `Created`, `Changed` or `Removed`. A rename is the old path removed and the new one created.
- Watching reads what is in the folder, so it needs `Fs.read`, and `uses Fs("…")` (8.1) limits it to its folders, as reading.
- `notify` runs on a thread of the runtime's own, in a crate of its own (`runtime/watch`), so a program that watches nothing links none of it (decision 0024).

## Consequences

- Hot reload and rebuild-on-save need no polling of file times.
- An editor saving a file may cause several changes; `poll()` gives each path once per call, in the order of its first change.
- Watching network drives and some container file systems may give no changes, as their systems give none.

## Alternatives considered

- **Kavach's own code for each system:** three system APIs and their quirks to keep up. Rejected for now.
- **Polling file times in Kavach:** no dependency, but slow for big folders and late by the polling interval. Rejected.
