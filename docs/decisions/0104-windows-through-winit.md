# 0104. `std.window`: windows through `winit`, behind `Window` and `Input`

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Kavach can compute on the GPU but can't show anything: it has no windows. Games, user interfaces (`std.ui`), tools and charts all need them. A maintainer plans a 3D game in Kavach whose first engine test (December 2026 to March 2027) draws a million particles in a window. That makes windows the first part of language request 9 in the to-do.

A window needs each system's own code: Win32 on Windows, X11 or Wayland on Linux, Cocoa on macOS. It also needs an event loop that hands over resizes, closes, keys and mouse moves. A maintainer chose the library, the capabilities and how events arrive (2026-11-14).

## Decision

- **`winit`** opens windows and runs their event loop on Windows, Linux (X11 and Wayland) and macOS. It is what `wgpu`, which Kavach already uses for the GPU, is made to work with, so a window can be drawn into later (language request 9 (2)).
- **Two capabilities** (spec 8.1):
  - `Window` opens windows, changes them, and reads their window events: resized, closed, focus.
  - `Input` reads the keyboard, the mouse and text typed in them.
  - A function that only draws needs no `Input`, so it can't read what a person types. Watching input outside the program's own windows would be a third capability, later.
- **Events as values the program reads**, as it reads a socket or a channel:
  - `win.events()` waits for at least one event and gives all that came;
  - `win.poll()` gives what came without waiting, for a game's frame loop.
  - A callback form, `win.run(f)`, calls `f` for each event until it returns `false` or the window closes. It is built on `events()`.
- The system's event loop runs on a thread of the runtime's own. The program's tasks never block it, and a slow frame can't freeze the window.

## Consequences

- Games, tools and `std.ui` get windows with no code per system in Kavach programs.
- `winit` is a large dependency with each system's libraries behind it. A program that doesn't use `std.window` links none of it (decision 0024).
- macOS requires the event loop on the process's first thread. Until the runtime gives that thread to `winit`, `std.window` on macOS is an open question in the spec.
- Drawing (surfaces, render pipelines), sound and gamepads come in later parts of language request 9, each with its own spec section.

## Alternatives considered

- **Kavach's own code for each system:** no dependency, but months of work and many system-specific bugs before the engine test.
- **One capability for windows and their input:** simpler, but every function that draws could also read the keyboard.
- **Only a callback** (`window.run(f)`, as `winit` and most C++ libraries have it): simple to build, but it hands control to the loop, so it doesn't mix with tasks and waiting. Kept as a second form.
