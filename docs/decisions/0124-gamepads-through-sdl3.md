# 0124. Gamepads and their gyroscopes through SDL3

- **Status:** Accepted
- **Date:** 2026-11-16
- **Replaces:** [decision 0113](0113-gamepads-through-gilrs.md)

## Context

`std.input` reads gamepads through `gilrs` (decision 0113). Motion controls need each pad's gyroscope: aiming with a DualSense or a Switch Pro pad. `gilrs` reads no gyroscopes, and its pad database and rumble cover fewer pads than SDL's, the library most games use for pads. A maintainer chose on 2026-11-16, from options with a recommendation: gamepads and their gyroscopes through SDL3, now.

## Decision

- **Gamepads come through SDL3**, by the `sdl3` crate (zlib licence), which builds SDL3 from its source with CMake and links it into the program. A program that reads no pads holds none of it (decision 0024). A Kavach program's author installs nothing: SDL3 is inside the runtime library `kavach build` links.
- **Building Kavach needs CMake.** When it is missing, Kavach's build stops at once and says how to install it on that system (`winget`, `brew`, `apt`, `dnf`, `pacman`), instead of failing deep inside SDL's build.
- The standard layout, the events and `input.pads()` stay as spec 14.26 has them. A pad's number is SDL's for it.
- **Gyroscopes** are each pad's state, `pad.gyro`, in radians a second, read once a frame as the sticks are. They are `none` for a pad that has none. They come as no events, which would arrive a thousand times a second.
- **Touch** comes from the window system through `winit` (spec 14.22), not SDL: SDL sees touch only on windows it made itself.
- SDL3 is built in release mode even in Kavach's debug builds, so it uses the C runtime every program links.

## Consequences

- More pads are known by name and layout, rumble works on more of them, and motion controls are possible.
- Building Kavach takes longer once (SDL's build) and needs CMake.
- Pad numbers differ from `gilrs`'s; programs never kept them.

## Alternatives considered

- **Touch through `winit` now, gyroscopes later:** the recommendation, which a maintainer chose against.
- **SDL3 installed on the system and loaded while running:** every player would need SDL3 installed. Rejected.
- **Gyroscopes from each system's own APIs:** a large amount of code per system and per pad. Rejected.
