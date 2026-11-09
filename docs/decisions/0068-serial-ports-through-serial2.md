# 0068. Serial ports through `serial2`

- **Status:** Accepted (2026-11-04)
- **Date:** 2026-11-04
- **Changes:** spec 8.1, 14.18; `runtime/core` gains a dependency

## Context

Item 6 of the to-do (Mithlesh, 2026-11-04) asks for modules for the devices connected to the machine, each behind a capability; USB, HID and serial devices are its fourth part. Serial ports are the commonest: Arduino and other boards, sensors, modems, lab instruments. Windows and Unix open and set them up differently (`CreateFile` and `DCB`; `open` and `termios`).

## Decision

- **Serial ports come through the `serial2` crate** (BSD-2-Clause or Apache-2.0, no system libraries needed). `serialport`, the best-known crate, was left out: it is MPL-2.0, a copyleft licence Mithlesh hasn't approved (Kavach is Apache-2.0 or MIT, decision 0011), and it needs `libudev` on Linux.
- **One capability per kind of device**: `Serial` now; `Usb`, `Hid` and `Camera` when they come, so a signature says exactly what a function can reach.
- **A port is a Kavach record holding a number**, and the runtime keeps the open ports by number, as an operating system keeps open files. The core gets no new kind of value (Mithlesh's rule that modules stay apart from the core).
- **A read waits at most a second** and gives what came. It holds the task's turn while it waits; letting other tasks run, as the network does, comes later (spec 14 open questions).

## Consequences

- A program can talk to a board over USB serial today, on Windows, Linux and macOS.
- It is tested on Linux with a pseudo-terminal standing in for a port; real hardware is not tested.

## Alternatives considered

- **`serialport`**: MPL-2.0, and `libudev` on Linux.
- **A new kind of value in the interpreter and runtime**, as sockets are. That would be faster to reach, but it changes the core for one module.
