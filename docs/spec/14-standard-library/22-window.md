# 14.22 std.window

Windows on the screen, and the keyboard and mouse input that comes to them. Games, tools, charts and user interfaces (`std.ui`, later) start here. Windows come through the `winit` crate on Windows, Linux (X11 and Wayland) and macOS ([decision 0104](../../decisions/0104-windows-through-winit.md)).

Two capabilities (8.1) keep them apart:

- `Window` opens windows, changes them, and reads their window events.
- `Input` reads the keyboard, the mouse and text typed in them, and gamepads (14.26).

A function that only draws needs no `Input`, so it can't read what a person types.

| Function or method | Gives |
|---|---|
| `window.open(title, width, height) -> AppWindow ! WindowError` | A new window of `width` × `height` pixels, shown at once |
| `win.events() -> [Event]` | The events that came, waiting for at least one |
| `win.poll() -> [Event]` | The events that came, without waiting; empty when none did |
| `win.run(f)` | Calls `f(event)` for each event as it comes, until `f` gives `false` or the window closes |
| `win.listen_input()` | From now on, keyboard, mouse, text and gamepad events come too. Needs `Input` |
| `win.size() -> (width: Int, height: Int)` | The inside of the window, in pixels |
| `win.scale() -> Float` | How many pixels make one point on this screen: `2.0` on most high-DPI screens |
| `win.set_title(title)` | Changes the title |
| `win.set_fullscreen(on)` | Fills the screen the window is on, or leaves it |
| `win.lock_cursor(on)` | Hides the mouse pointer and keeps it in the window, as a game's camera wants; `Motion` events still come. Needs `Input` |
| `win.number() -> Int` | The window's number, which `std.gpu` (14.24) draws into it by |
| `win.close()` | Closes the window; a window is closed anyway when its last holder ends |

`Event` is a union:

| `Event` | When |
|---|---|
| `Closed` | The person asked to close the window. It stays open until `close()` or its last holder ends, so a program can ask "save first?" |
| `Resized(width: Int, height: Int)` | Its inside has this size now, in pixels |
| `Scaled(factor: Float)` | It moved to a screen with another scale |
| `Focused(on: Bool)` | It gained or lost the keyboard |
| `Key(key: String, down: Bool)` | A key went down or up. `key` names the key's place, not its letter, as the W3C's `KeyboardEvent.code` does: `KeyW` is the same key on every keyboard layout, as a game wants. Keys held down don't repeat |
| `Text(text: String)` | Text typed, after the keyboard layout and input methods (IME), as a text box wants; held keys repeat here |
| `MouseMoved(x: Float, y: Float)` | The pointer is here, in pixels from the window's top left |
| `MouseButton(button: Int, down: Bool)` | A button went down or up: 0 left, 1 right, 2 middle, 3 back, 4 forward |
| `Wheel(x: Float, y: Float)` | The wheel turned, in lines; `y` above 0 is away from the person |
| `Motion(x: Float, y: Float)` | The mouse moved this much, however the pointer moved or stopped: for a game's camera |
| `PadConnected(pad: Int, name: String)` | A gamepad is connected, or was when the window began to listen: its number and its name (14.26) |
| `PadGone(pad: Int)` | The gamepad was disconnected |
| `PadButton(pad: Int, button: String, down: Bool)` | A button of the standard layout went down or up, named by its place: `South`, `Start`, `DPadUp` (14.26) |
| `PadAxis(pad: Int, axis: String, value: Float)` | A stick or trigger moved: `LeftX`, `LeftY`, `RightX`, `RightY` from -1 to 1, `LeftTrigger`, `RightTrigger` from 0 to 1 |

The events from `Key` on come only after `listen_input()`. Gamepad events come to every window that listens to input.

`WindowError` is `Unavailable(reason: String)`: the system shows no windows here, as on a server with no screen.

```kavach
use std.window

fn main() ! WindowError uses Window, Input, Console {
    const win = try window.open("Swarm", 1280, 720)
    win.listen_input()
    let running = true
    while running {
        for e in win.events() {
            match e {
                Event.Closed => running = false,
                Event.Key(key, down) => if down and key == "Escape" { running = false },
                Event.Resized(w, h) => print("now {w} × {h}"),
                _ => {}
            }
        }
    }
}
```

A game reads `win.poll()` once a frame instead, so it never waits for the person. `run` is for programs that only answer events:

```kavach
win.run(e => match e {
    Event.Key(key, down) => not (down and key == "Escape"),   // false ends `run`
    _ => true,
})
```

- The system's event loop runs on a thread of the runtime's own, so a slow frame never freezes a window. `events()` holds the task's turn while it waits (9.7).
- `window.open_id`, `events_id`, `texts_id`, `listen_id`, `size_id`, `scale_id`, `set_title_id`, `set_fullscreen_id`, `lock_cursor_id` and `close_id` are what `open` and the methods are built on, by the window's number.
- Generated tests (11.5) skip functions that use `Window` or `Input`: there is no fake screen yet.

## Open questions

- macOS shows windows only from a program's first thread. Until the runtime gives that thread to the event loop, `window.open` there fails with `Unavailable`.
- Touch screens and pens, with `std.input` (14.26). Drawing into a window is `std.gpu` (14.24).
