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

## Open questions

- Each GPU's memory, and the memory a GPU has free.
- Changes while the program runs: memory available now, the processor's current speed, its temperature.
