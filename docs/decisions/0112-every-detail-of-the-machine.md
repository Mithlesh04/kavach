# 0112. Every detail of the machine, then its events and controls

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

`std.system` (spec 14.25, [decision 0110](0110-std-system.md)) says what system, processor, memory and GPUs a program runs on. A maintainer asked for much more (2026-11-15): "if a system engineer, application engineer, network engineer or embedded engineer uses Kavach, they should not get any kind of difficulty: they should get all the details and controls". The list: the motherboard, the BIOS and the machine's model; RAM modules and their slots; disks (SSD or HDD), volumes with their used and available space, external drives and pen drives; network adapters and their drivers, Wi-Fi networks; Bluetooth; speakers and microphones; displays and the ports they use (HDMI, DisplayPort); keyboards and mice; fans and temperatures; the battery and charging; installed applications and drivers; serial numbers.

A maintainer chose (2026-11-15): all of these; serial numbers and hardware ids behind a capability of their own, stronger than `System`.

Every system keeps these in its own place. Windows has the registry, the SMBIOS tables (`GetSystemFirmwareTable`), the device list of SetupAPI, the IP helper and the display configuration; Linux has `/sys` and `/proc`; macOS has `sysctl` and IOKit. None of it needs a new outside library: `windows-sys` and `libc` already reach all of them (decision 0110).

## Decision

- **Three stages**, each with its spec section: (1) **details**, read-only; (2) **events**, a device added or removed, a volume mounted, the power changing, as a channel (9.4); (3) **controls**, things that change the machine.
- **Details are typed records** for what people ask about most (memory modules, disks, volumes, network adapters, displays, the battery, sensors, the board, the BIOS, the model), and **one list of every device** the system knows (`system.devices()`: its kind, name, maker, driver and whether it is connected), so a Bluetooth adapter, a speaker, a keyboard or a camera needs no function of its own.
- **Capabilities** (8.1), each listed apart by `kavach audit` and given to an agent only with a person's approval. Unlike `Fs`, which covers `Fs.read` and `Fs.write`, `uses System` covers none of the parts below: each is written by name, so reading the details never grants reading serial numbers or switching Wi-Fi off. `main` has them all, as it has every capability:
  - `System`: the details, as for `memory` and `cpu` now.
  - `System.ids`: serial numbers, system UUIDs and MAC addresses, which name one exact machine.
  - `System.apps`: the applications installed, which say much about the person using the machine (a choice of this record; a maintainer may move it into `System`).
  - Controls, one capability each, as they change the machine for everyone on it: `System.audio` (volume, default speaker), `System.display` (brightness), `System.radio` (Wi-Fi and Bluetooth on or off), `System.storage` (ejecting a drive), `System.power` (sleep, shutdown).
- **Nothing fails for want of a detail**: what a system doesn't give is `""`, 0 or an empty list, and each spec row says which systems give it. What needs admin rights (RAM modules on Linux, serial numbers on Linux) says so, and gives nothing without them.
- **The code lives in `runtime/process`**, beside 0110's facts, one file per area and system.

## Consequences

- An engineer gets the machine's inventory from one module, with types, on every system Kavach runs on, and a reviewer sees from `uses` which programs read what.
- The platform code grows a lot, each area a small file per system, with `unsafe` calls only where the system's API needs them, each with a `SAFETY:` comment.
- macOS gives less at first (the model, disks and volumes, the battery); IOKit comes later.
- Ports are known by what is plugged into them: a display's connection (HDMI, DisplayPort, internal), a USB device's port. An empty port is reported only where the system says so.

## Alternatives considered

- **The `sysinfo` crate.** It covers processes, disks, networks and sensors, but not SMBIOS memory modules, the device list, displays or applications, and it is a large dependency.
- **WMI on Windows.** It answers most questions, but through COM, slowly, and some classes need admin rights; the registry, SetupAPI and the SMBIOS tables give the same without them.
- **One capability for everything.** Simpler, but a program sizing its threads would ask for the right to read serial numbers and switch Wi-Fi off.
