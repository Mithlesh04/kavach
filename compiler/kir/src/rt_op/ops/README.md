# compiler/kir/src/rt_op/ops

The built-in operations (`RtOp`), one list per family. `compiler/kir/build.rs` reads them in the order below and makes the `RtOp` enum and `RtOp::ALL` from them: an op's code, which native code passes to the runtime, is its place in these lists. Each op is a line `Name,`, after its doc comment (`///`). A new op goes at the end of its family's list.

The runtime entry that carries out an op is chosen by its name's first word (`RtOp::entry` in `../groups.rs`), not by its file.

| File | Ops |
|---|---|
| `text.ops` | Text, numbers, regular expressions, panics, `std.base64` and `std.compress` |
| `collections.ops` | Lists, maps, sets, bytes, `Shared`, raw memory and drops |
| `json.ops` | `std.json`: building, reading, encoding and decoding |
| `system.ops` | Files, the console, the command line, time, logs, the clipboard, serial ports, windows, processes and random numbers |
| `net.ops` | `std.net` and `std.crypto` |
| `tasks.ops` | Tasks and channels |
| `agents.ops` | Agents, their journals and `ask` |
| `gpu.ops` | The GPU: computing, and drawing into windows (`std.gpu`) |
