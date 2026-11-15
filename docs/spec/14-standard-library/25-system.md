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
| `system.apps() -> [App]` | The applications installed: `App(name:, version:, maker:)`. Needs `System.apps` | Windows, Linux (packages) |
| `system.ids() -> Ids` | `Ids(system_uuid:, board_serial:, macs:)`: what names this exact machine. Needs `System.ids` | Windows; Linux for root |

- Every function here needs `System`, except `apps` and `ids`, which need their own capabilities (8.1). `uses System` doesn't cover them, as it covers no other part of `System`'s: a function that reads ids writes `uses System.ids`. Each is listed apart by `kavach audit`, and an agent gets one only with a person's approval (8.3).
- The lists are in the system's order. Each call reads the machine again: a pen drive plugged in shows in the next call.

## Open questions

- Stage 2 of decision 0112: events, a device added or removed, a volume mounted, the power changing, as a channel (9.4).
- Stage 3: controls, each behind its own capability: `System.audio` (volume, the default speaker), `System.display` (brightness), `System.radio` (Wi-Fi and Bluetooth on or off), `System.storage` (ejecting a drive), `System.power` (sleep, shutdown).
- The Wi-Fi networks in range, and each USB port with what is plugged into it.
- macOS through IOKit: disks, displays, devices and sensors.
- Each GPU's memory, and the memory a GPU has free.
- Changes while the program runs: memory available now, the processor's current speed, its temperature.
