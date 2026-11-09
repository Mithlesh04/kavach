# 14.18 std.devices

Devices connected to the machine, each kind behind a capability of its own (8.1), so a function's signature shows what it can reach and an agent gets one only when a person approves it. Serial ports come first: the boards, sensors and modems on a USB or RS-232 cable. Ports need the capability `Serial`.

| Function or method | Gives |
|---|---|
| `devices.serial_ports() -> [String]` | The serial ports there are: `COM3` on Windows, `/dev/ttyUSB0` on Linux |
| `devices.open_serial(path, baud) -> SerialPort ! DeviceError` | Opens the port at `path` at `baud` bits a second (8 data bits, no parity, 1 stop bit) |
| `port.write(data) ! DeviceError` | Sends the `Bytes` `data` |
| `port.read(max) -> Bytes ! DeviceError` | Up to `max` bytes: what has arrived, waiting at most a second for the first; empty `Bytes` when none came |
| `port.close()` | Closes the port; a port is closed anyway when its last holder ends |

`DeviceError` is `NotFound(path)`, `Denied(path)` (another program has the port, or the user may not open it) or `Failed(path, reason)`.

```kavach
use std.devices

fn main() ! Error uses Serial, Console {
    for path in devices.serial_ports() {
        print(path)
    }
    const board = try devices.open_serial("/dev/ttyUSB0", 115_200)
    try board.write("ping\n".bytes())
    const answer = try board.read(64)
    print(answer.text() ?? "?")
}
```

- `devices.open_serial_id(path, baud)`, `read_serial(id, max)`, `write_serial(id, data)` and `close_serial(id)` are what `open_serial` and the methods are built on, by the port's number.
- A wait for bytes holds the task's turn (9.7), for at most a second.
- Ports come through the `serial2` crate ([decision 0068](../../decisions/0068-serial-ports-through-serial2.md)). USB, HID, cameras and the rest of item 6 of the to-do come after.
