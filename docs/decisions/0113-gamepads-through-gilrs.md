# 0113. Gamepads through `gilrs`, as events and as state

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

Games need gamepads. `std.window` already gives the keyboard, the mouse, raw mouse motion and a locked cursor as window events, behind `Input` (spec 14.22, [decision 0104](0104-windows-through-winit.md)). It gives no gamepads, and `winit` has none.

Each system reads gamepads its own way: Windows.Gaming.Input or XInput on Windows, evdev through udev on Linux, IOKit on macOS. Pads also differ in how their buttons are numbered, so a program needs one standard layout (a south button, an east button, two sticks, two triggers), whatever pad is plugged in. This is language request 9 (3) in the to-do. A maintainer chose the library and the shape of the API (2026-11-15).

## Decision

- **`gilrs`** (MIT or Apache-2.0) reads gamepads on Windows, Linux and macOS. It maps each pad to one standard layout through SDL's mapping database, and it can make pads rumble.
- **Both ways a program wants them:**
  - as **events** in the window's stream, like keys: a pad connected or gone, a button down or up, a stick or trigger moved. They come to every window that listens to input (`listen_input()`), which already needs `Input`;
  - as **state**: `input.pads()` gives each connected pad now, its buttons held and where its sticks and triggers are, for a game loop that reads it once a frame.
- Buttons and axes are named by their **place** in the standard layout (`South`, `East`, `LeftX`), not by the letter printed on them, as keys are named by `KeyboardEvent.code` (14.22). A game reads the same button on an Xbox pad and a PlayStation pad.
- `pad.rumble(strong, weak, ms)` makes a pad vibrate. It is output to a device the person holds, so it needs `Input` too, not a capability of its own.
- `gilrs` runs on a thread of the runtime's own, which keeps every pad's state and passes events to the windows. The program never waits for it.
- **On Linux, `gilrs` links `libudev`.** Only a program that reads gamepads links it (decision 0024): `kavach build` adds `libudev.so.1` for such a program. Every desktop Linux with systemd has it. Building Kavach itself on Linux needs `libudev-dev` (or its files) for the `pkg-config` check of `libudev-sys`.

## Consequences

- Games get gamepads in the standard layout, with rumble, and no code for each system.
- A machine with no gamepad, or where `gilrs` can't start (no udev, as in a container), has no pads: `input.pads()` is empty and no pad events come. That is not an error.
- Gamepads can't be tested without one connected. Tests check the names, the layout and the state kept from events, with events made up in the test.

## Alternatives considered

- **Kavach's own code for each system:** XInput on Windows and evdev on Linux through `windows-sys` and `libc`, macOS later. No new dependency, but XInput sees only Xbox-style pads, every pad needs its own mapping, and the code is ours to keep up. Rejected for now.
- **SDL3 through C:** the widest support (PlayStation, Switch and Xbox layouts, rumble, gyroscopes), but a large C library to build and ship with every program that reads pads. Rejected for its weight.
- **State only, or events only:** state only makes a menu that waits for a button poll; events only makes every game keep the same state itself. A maintainer chose both.
