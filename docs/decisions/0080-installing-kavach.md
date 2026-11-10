# 0080. Installing Kavach

- **Status:** Accepted
- **Date:** 2026-11-10

## Context

Today Kavach runs only where the repository is built with cargo. Outside developers (to-do: "Kavach installed the way each system installs software") need a ready-built Kavach they install the way their system installs software, and `kavach build` should make fully optimised programs out of the box.

What a build needs at run time sets what an install must hold: the `kavach` executable (the CLI carries the formatter and the language server), the runtime library `kavach build` links (next to the executable, decision 0016), LLVM's `opt` and `llc` for optimised builds (decision 0018, falling back to Cranelift without them), and a linker with the system's C libraries, which Kavach does not ship.

## Decision

A maintainer decided (2026-11-10):

- **One install holds everything but the linker:** `kavach` (and the `kv` alias), the runtime library, and LLVM's `opt` and `llc` (about 100 MB, Apache-2.0 with the LLVM exception), laid out so `kavach` finds them next to itself: `bin/kavach`, `lib/kavach_runtime_native.lib` (`.a` elsewhere), `llvm/opt`, `llvm/llc`. The tool lookup learns this place, before `KAVACH_LLVM_BIN` and rustup.
- **The linker is checked, not bundled:** the installer and `kavach build` detect a missing linker and say exactly what to install (Visual Studio Build Tools on Windows, `cc` on Linux, Xcode's command-line tools on macOS). Bundling `lld` may come later, as decision 0016 foresaw.
- **Archives and install scripts come first:** a release archive per system and architecture (Windows, Linux, macOS; x86-64 and ARM64) with checksums, and an install script per system (PowerShell on Windows, shell elsewhere) that unpacks to the user's tools folder and adds `kavach` to the path. Then, in later work: `winget`, Homebrew, `.deb` and `.rpm`, a Windows installer and a macOS `.pkg`, and signing.
- **CI builds the archives** on manual dispatch (CI is manual-only for now). Publishing a release is outward-facing: a maintainer gives the go each time.

## Consequences

- Anyone can install Kavach without Rust, rustup or cargo, and `kavach build` makes optimised programs at once; only a linker may need installing, and the message says how.
- The download is about 100 MB larger for the LLVM tools; the alternative below was rejected.
- The tool and runtime lookups get one more place to search, with tests.
- The repository gains an install script per system in `scripts/`, and a CI job that builds the archives.

## Alternatives considered

- **LLVM tools as an optional download:** a small first download, but most users would silently get the slower Cranelift backend, and the first "why is this slow?" costs more than 100 MB.
- **Users install rustup's `llvm-tools` themselves:** what developers of Kavach do today; too many steps for users of Kavach.
- **Bundling a linker (`lld`) now:** removes the last manual step, but on Windows the system libraries must still come from the Build Tools, so it doesn't free the user from installing them; revisit later.
- **Package managers first:** `winget` and Homebrew reach many users, but each needs a published, versioned release to point at, so the archives must exist first anyway.
