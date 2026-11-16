# 0120. An icon and a manifest for Windows executables, written by Kavach

- **Status:** Accepted
- **Date:** 2026-11-16

## Context

A game or desktop tool built with `kavach build` showed Windows' default icon, and without a manifest Windows blurs its window on high-DPI screens and limits its paths to 260 characters. Both live in an executable's resources, which Windows tools put there with the SDK's `rc.exe`. A maintainer chose, 2026-11-16, that Kavach write them itself (to-do language request 9 (12)).

## Decision

- `kavach build` on Windows writes a resource file (`.res`) and gives it to the linker with the program. It always holds a manifest: run as the user who starts it, per-monitor DPI awareness, long paths, Windows 10 and later.
- With `[build] icon = "file.png"` in `kavach.toml` it also holds the icon. Windows has read PNG icons since Vista, so the PNG goes in as it is: an icon resource and the group that names it. The PNG is checked: its signature, and a size of at most 256 × 256.
- The `.res` format is written by Kavach's own code (`compiler/backend/cranelift/src/resources.rs`), so no Windows SDK and no new dependency is needed; `link.exe` turns it into the executable's resources.

## Consequences

- Built programs have an icon and look sharp, with nothing to install.
- An icon is one size; Windows scales it for smaller places. Several sizes in one icon wait until a need shows.
- Other systems are not changed: Linux and macOS give a program its icon outside the executable.

## Alternatives considered

- **The `embed-resource` crate:** less code of ours, but it runs `rc.exe` from the Windows SDK, which a machine with only the Build Tools may lack. Rejected by a maintainer.
- **Changing the built executable with `UpdateResourceW`:** no linker change, but system calls in the command-line tool, which allows no `unsafe` code. Rejected.
- **Waiting:** every built program would keep the default icon. Rejected by a maintainer.
