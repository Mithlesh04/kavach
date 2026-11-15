# 14.25 std.system

What machine the program runs on: its system, processor, memory and GPUs. A program uses it to size its work (how many threads, how big a batch, whether a GPU pays off), to write a bug report or a benchmark's header, and to record where a run happened ([decision 0110](../../decisions/0110-std-system.md)).

The basics are free to read, like the clock (8.1): the system's name and version, the architecture and the number of cores. They identify no one. The rest needs the `System` capability, since together they can tell one machine from another: `kavach audit` lists it, and an agent gets it only when a person approves (8.3).

```kavach
use std.system

fn threads() -> Int {
    system.cores().logical
}

fn header() -> String uses System {
    const os = system.os()
    const cpu = system.cpu()
    const memory = system.memory()
    "{os.name} {os.version} on {system.arch()}, {cpu.model}, {memory.total / 1_000_000_000} GB"
}
```

| Function | Gives | Needs |
|---|---|---|
| `system.os() -> Os` | `Os(name:, version:)`: `"windows"`, `"linux"`, `"macos"` or another system's name, and its version as the system writes it (`"10.0.26300"`, `"6.6.87"`) | nothing |
| `system.arch() -> String` | The processor's architecture: `"x86_64"`, `"aarch64"` or another | nothing |
| `system.cores() -> Cores` | `Cores(logical:, physical:)`: how many threads can run at once for this program, and the processor cores behind them; `physical` is `logical` where the system doesn't say | nothing |
| `system.memory() -> Memory` | `Memory(total:, available:)` in bytes: the machine's memory, and how much a program could still take without the system swapping | `System` |
| `system.cpu() -> Cpu` | `Cpu(model:, vendor:, mhz:)`: the processor's name (`"Intel(R) Core(TM) i7-13700HX"`), its maker (`"GenuineIntel"`, `"AuthenticAMD"`, `"Apple"`) and its rated speed; `""` and 0 where the system doesn't say | `System` |
| `system.gpus() -> [GpuInfo]` | Each GPU the program could use, discrete ones first, then integrated, the rest last: `GpuInfo(name:, vendor:, kind:, backend:)`, `kind` a `GpuKind` (`Discrete`, `Integrated`, `Virtual`, `Other`, or `Software` for one that draws on the processor), `backend` the API it is reached through (`"vulkan"`, `"dx12"`, `"metal"`, `"gl"`). Listing them opens no GPU | `System` |
| `system.hostname() -> String` | The machine's name on its network | `System` |
| `system.uptime() -> Duration` | How long since the machine started (14.4) | `System` |

- Nothing here fails: what a system doesn't say comes back as `""` or 0.
- `system.gpus()` gives no memory size: the GPU interfaces Kavach uses don't report it.
- The program's own processor time is `process.cpu_ms()` (14.23), beside `process.peak_bytes()`.

## Every part of the machine

Each part an engineer may need, read without admin rights where the system allows ([decision 0112](../../decisions/0112-every-detail-of-the-machine.md)). What a system doesn't give is `""`, 0 or an empty list, never an error; the last column says where each comes from.

```kavach
fn report() uses System {
    for v in system.volumes() {
        print("{v.path}: {v.available / 1_000_000_000} of {v.total / 1_000_000_000} GB free{if v.removable { ", removable" } else { "" }}")
    }
    if let b = system.battery() {
        print("battery {b.percent}%, {if b.charging { "charging" } else { "on battery" }}")
    }
    for d in system.devices().filter(d => d.kind == "bluetooth") {
        print("{d.name} ({d.maker})")
    }
}
```

| Function | Gives | Where |
|---|---|---|
| `system.board() -> Board` | `Board(maker:, product:, version:)`: the motherboard | Windows, Linux |
| `system.bios() -> Bios` | `Bios(maker:, version:, date:)`: the firmware | Windows, Linux |
| `system.model() -> MachineModel` | `MachineModel(maker:, product:, family:)`: the machine itself (a laptop's name, a server's model) | Windows, Linux, macOS (`product` only) |
| `system.memory_modules() -> [MemoryModule]` | Each memory slot: `MemoryModule(slot:, size:, kind:, mhz:, maker:, part:)`; an empty slot has `size` 0. `kind` is `"DDR4"`, `"DDR5"` or another | Windows; Linux for root |
| `system.disks() -> [Disk]` | Each physical drive: `Disk(name:, model:, size:, kind:, removable:)`, `kind` a `DiskKind` (`Ssd`, `Hdd`, `Usb`, `Other`) | Windows, Linux |
| `system.volumes() -> [Volume]` | Each mounted volume: `Volume(path:, label:, file_system:, total:, available:, removable:)`, sizes in bytes; a pen drive is a removable volume | Windows, Linux, macOS |
| `system.network() -> [Adapter]` | Each network adapter: `Adapter(name:, description:, kind:, connected:, mbps:, addresses:)`, `kind` an `AdapterKind` (`Ethernet`, `WiFi`, `Bluetooth`, `Loopback`, `Other`), `addresses` its IP addresses | Windows, Linux |
| `system.displays() -> [Display]` | Each display connected: `Display(name:, width:, height:, hz:, connection:)`, `connection` the port it uses: `"HDMI"`, `"DisplayPort"`, `"internal"`, `"VGA"`, `"DVI"` or `""` | Windows, Linux |
| `system.battery() -> Battery?` | `Battery(percent:, charging:, on_power:, minutes_left:)`, or `none` with no battery; `minutes_left` is -1 while charging or not known | Windows, Linux, macOS |
| `system.sensors() -> [Sensor]` | Each sensor: `Sensor(name:, kind:, value:)`, `kind` a `SensorKind` (`Temperature` in °C, `Fan` in RPM, `Voltage` in volts, `Power` in watts) | Linux; Windows temperatures |
| `system.devices() -> [Device]` | Every device the system knows: `Device(kind:, name:, maker:, driver:, driver_version:, connected:)`, `kind` one of `"bluetooth"`, `"network"`, `"audio"`, `"display"`, `"gpu"`, `"keyboard"`, `"mouse"`, `"storage"`, `"usb"`, `"camera"`, `"printer"`, `"processor"`, `"other"` | Windows, Linux |
| `system.drivers() -> [Driver]` | The drivers loaded: `Driver(name:, version:)` | Windows, Linux |
| `system.apps() -> [App]` | The applications installed: `App(name:, version:, maker:)`. Needs `System.critical.read` | Windows, Linux (packages) |
| `system.ids() -> Ids` | `Ids(system_uuid:, board_serial:, macs:)`: what names this exact machine. Needs `System.critical.read` | Windows; Linux for root |

- Every function here needs `System`, except `apps` and `ids`, which need `System.critical.read` (8.1). `uses System` doesn't cover `System.critical` or its parts, `System.critical.read` and `System.critical.write`: they are written by name, `kavach audit` lists them apart, and an agent gets one only with a person's approval (8.3).
- The lists are in the system's order. Each call reads the machine again: a pen drive plugged in shows in the next call.

## Programs, ports and users

The programs running, the network ports they hold, and the system's users, for the engineers who look after a machine ([decision 0112](../../decisions/0112-every-detail-of-the-machine.md)). Changing any of them needs `System.critical.write`, and the system's own permission too: a program not run as an administrator, or as root, can end only its own user's programs and can't add or remove users. A change refused fails with `SystemError.Denied`.

```kavach
fn free_8080() ! SystemError uses System.critical.read, System.critical.write, Console {
    for p in system.ports().filter(p => p.port == 8080 and p.state == "listen") {
        print("8080 is held by {p.program} ({p.pid})")
    }
    const ended = try system.free_port(8080)
    print("ended {ended.len()} programs")
}
```

| Function | Gives | Needs |
|---|---|---|
| `system.running() -> [RunningApp]` | Each program running: `RunningApp(pid:, name:, path:, memory:)`, `memory` its resident bytes; `path` is `""` where the system doesn't show it | `System.critical.read` |
| `system.ports() -> [PortUse]` | Each port in use: `PortUse(protocol:, address:, port:, state:, pid:, program:)`, `protocol` `"tcp"` or `"udp"`, `state` `"listen"`, `"established"` or another (`""` for UDP) | `System.critical.read` |
| `system.terminate(pid, force: false) ! SystemError` | Ends program `pid`: asked to stop, or, with `force`, stopped at once (Windows always stops it at once) | `System.critical.write` |
| `system.free_port(port, force: false) -> [Int] ! SystemError` | Ends every program listening on `port`, and gives their pids | `System.critical.write` |
| `system.user() -> User` | The user this program runs as: `User(name:, home:, admin:)`, `admin` whether it runs with administrator or root rights now | `System` |
| `system.users() -> [Account]` | Every account a person logs in with: `Account(name:, full_name:, admin:)` | `System.critical.read` |
| `system.add_user(name, password, admin: false) ! SystemError` | Makes a user account; `password` may be a secret (8.7) | `System.critical.write` |
| `system.remove_user(name) ! SystemError` | Deletes a user account and its home folder | `System.critical.write` |
| `system.lock()` | Locks the session, so another user can log in | `System.critical.write` |
| `net.free_port() -> Int ! NetError` | A TCP port nothing listens on now, chosen by the system | `Net` |
| `net.is_free(port) -> Bool` | Whether this program could listen on `port` now | `Net` |

- `SystemError` is `NotFound(what:)`, `Denied(reason:)` or `Failed(reason:)`.
- A port found free can be taken by another program before this one listens on it: a server that can should listen on port 0 and read the port it got (14.10).
- Their tests change the machine, so they run only in a throwaway container: the tests in the repository check what is refused without the rights.

## Open questions

- Running a program as another user (`process.start` with a user and password), and switching the logged-in user without locking first.
- Stage 2 of decision 0112: events, a device added or removed, a volume mounted, the power changing, as a channel (9.4).
- Stage 3: controls, behind `System.critical.write`: volume and the default speaker, brightness, Wi-Fi and Bluetooth on or off, ejecting a drive, sleep and shutdown.
- The Wi-Fi networks in range, and each USB port with what is plugged into it.
- macOS through IOKit: disks, displays, devices and sensors.
- Each GPU's memory, and the memory a GPU has free.
- Changes while the program runs: memory available now, the processor's current speed, its temperature.
