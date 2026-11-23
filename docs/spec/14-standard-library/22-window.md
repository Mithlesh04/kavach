# 14.22 std.window

Windows on the screen, and the keyboard and mouse input that comes to them. Games, tools, charts and user interfaces (`std.ui`, later) start here. Windows come through the `winit` crate on Windows, Linux (X11 and Wayland) and macOS ([decision 0104](../../decisions/0104-windows-through-winit.md)).

Two capabilities (8.1) keep them apart:

- `Window` opens windows, changes them, and reads their window events.
- `Input` reads the keyboard, the mouse and text typed in them, and gamepads (14.26).

A function that only draws needs no `Input`, so it can't read what a person types.

| Function or method | Gives |
|---|---|
| `window.open(title, width, height) -> AppWindow ! WindowError` | A new window of `width` × `height` pixels, shown at once |
| `win.events() -> [Event]` | The events that came, waiting for at least one while other tasks run (9.7) |
| `win.poll() -> [Event]` | The events that came, without waiting; empty when none did |
| `win.run(f)` | Calls `f(event)` for each event as it comes, until `f` gives `false` or the window closes |
| `win.wake()` | Makes the window's waiting `events()` give a `Woken` event, from any task: a task that finished work, or a timer, tells the window's loop to look |
| `win.listen_input()` | From now on, keyboard, mouse, text and gamepad events come too. Needs `Input` |
| `win.size() -> (width: Int, height: Int)` | The inside of the window, in pixels; `(0, 0)` once `close()` was called |
| `win.scale() -> Float` | How many pixels make one point on this screen: `2.0` on most high-DPI screens |
| `win.set_title(title)` | Changes the title |
| `win.set_fullscreen(on)` | Fills the screen the window is on, or leaves it |
| `win.set_text_input(on)` | Lets input methods (IME) compose text in the window: from now on `Composing` events come while a person composes, and `Text` when they finish. Off at first, so a game's keys never open an input method. Needs `Input` |
| `win.set_caret(x, y, width, height)` | Where the text being typed is, in pixels from the window's top left, so the input method shows its list of choices beside it. Needs `Input` |
| `win.lock_cursor(on)` | Hides the mouse pointer and keeps it in the window, as a game's camera wants; `Motion` events still come. Needs `Input` |
| `win.number() -> Int` | The window's number, which `std.gpu` (14.24) draws into it by |
| `win.close()` | Closes the window; a window is closed anyway when its last holder ends. After it, the window has no events left to give |

`Event` is a union:

| `Event` | When |
|---|---|
| `Closed` | The person asked to close the window. It stays open until `close()` or its last holder ends, so a program can ask "save first?" |
| `Resized(width: Int, height: Int)` | Its inside has this size now, in pixels |
| `Scaled(factor: Float)` | It moved to a screen with another scale |
| `Focused(on: Bool)` | It gained or lost the keyboard |
| `Key(key: String, down: Bool, name: String = "", repeat: Bool = false, modifiers: Modifiers = Modifiers())` | A key went down or up. `key` names the key's place, not its letter, as the W3C's `KeyboardEvent.code` does: `KeyW` is the same key on every keyboard layout, as a game wants. `name` is what the key means in the keyboard's layout, as `KeyboardEvent.key` gives it: `z` or `Z`, `Enter`, `ArrowLeft`, `Backspace`, ` ` for the space bar. A key held down comes again and again with `repeat` true, as a text box wants; a game skips those |
| `Text(text: String)` | Text typed, after the keyboard layout and input methods (IME), as a text box wants; held keys repeat here |
| `MouseMoved(x: Float, y: Float, modifiers: Modifiers = Modifiers())` | The pointer is here, in pixels from the window's top left |
| `MouseButton(button: Int, down: Bool, clicks: Int = 1, modifiers: Modifiers = Modifiers())` | A button went down or up: 0 left, 1 right, 2 middle, 3 back, 4 forward. `clicks` counts quick presses in one place: 2 for a double click, 3 for a triple click (below) |
| `Wheel(x: Float, y: Float, pixels: Bool = false, phase: String = "Moved", modifiers: Modifiers = Modifiers())` | The wheel turned, in lines, or in pixels when `pixels` is true, as a touchpad scrolls; `y` above 0 is away from the person. `phase` is `Began`, `Moved` or `Ended` for a touchpad's gesture, `Moved` for a wheel |
| `Motion(x: Float, y: Float)` | The mouse moved this much, however the pointer moved or stopped: for a game's camera |
| `PadConnected(pad: Int, name: String)` | A gamepad is connected, or was when the window began to listen: its number and its name (14.26) |
| `PadGone(pad: Int)` | The gamepad was disconnected |
| `PadButton(pad: Int, button: String, down: Bool)` | A button of the standard layout went down or up, named by its place: `South`, `Start`, `DPadUp` (14.26) |
| `PadAxis(pad: Int, axis: String, value: Float)` | A stick or trigger moved: `LeftX`, `LeftY`, `RightX`, `RightY` from -1 to 1, `LeftTrigger`, `RightTrigger` from 0 to 1 |
| `Touch(id: Int, x: Float, y: Float, phase: String, pressure: Float)` | A finger or pen on a touch screen, in pixels from the window's top left. `phase` is `Began`, `Moved`, `Ended` or `Cancelled` (the system took the touch away); `id` is the same from `Began` to its end, so several fingers are told apart. `pressure` is from 0 to 1, or 1 where the screen doesn't measure it |
| `Redraw` | The system asks for the window to be drawn again: it was shown, uncovered or resized |
| `Woken` | `win.wake()` was called |
| `MouseEntered` | The pointer came into the window |
| `MouseLeft` | The pointer left the window |
| `Composing(text: String, from: Int, to: Int)` | An input method is composing `text`, not yet typed: show it where the caret is, underlined. The part from character `from` to `to` is the one it works on, where its caret is when they are equal; both are -1 when it shows none. An empty `text` ends the composing: the `Text` typed, if any, comes next |

The events from `Key` on, but `Redraw` and `Woken`, come only after `listen_input()`. Gamepad events come to every window that listens to input.

`Modifiers` is a record of four `Bool`s, each `false` by default: `shift`, `ctrl`, `alt`, and `meta` (the Windows key, or Command on a Mac). It says which were held when the key, button, move or wheel came: a `Key` going down whose `name` is `z` and whose `modifiers.ctrl` is true is Ctrl+Z, on any layout.

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
                Event.Resized(w, h) => print(`now {w} × {h}`),
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

- The system's event loop runs on a thread of the runtime's own, so a slow frame never freezes a window.
- `window.open_id`, `events_id`, `texts_id`, `listen_id`, `size_id`, `scale_id`, `set_title_id`, `set_fullscreen_id`, `lock_cursor_id`, `wake_id`, `text_input_id`, `caret_id` and `close_id` are what `open` and the methods are built on, by the window's number.
- Generated tests (11.5) skip functions that use `Window` or `Input`: there is no fake screen yet.

## 14.22.1 Waking and drawing again

A program that only answers events sleeps in `events()` until something happens, so it costs no power while nothing does. Other tasks run while it waits (9.7), as they do while a task waits for a socket. When one of them has something to show, it calls `win.wake()`, and the waiting `events()` gives a `Woken` event:

```kavach
use std.window
use std.time

fn main() ! WindowError uses Window, Console, Clock {
    const win = try window.open("Clock", 320, 200)
    const _ticks = spawn {
        for _ in 0..3 {
            time.sleep_ms(1000)
            win.wake()                  // the loop below looks again
        }
    }
    let woken = 0
    while woken < 3 {
        for e in win.events() {
            match e {
                Event.Woken => {
                    woken += 1
                },
                Event.Redraw => print("draw the window"),
                _ => {}
            }
        }
    }
}
```

- `wake()` may be called from any task, and from a task on another core (9.8). Several wakes before the events are taken give one `Woken`, and so do several `Redraw`s: each says "look again", not how many times.
- `wake()` on a closed window does nothing.
- `Redraw` comes when the system needs the window's picture: when it is first shown, uncovered or resized. A program that draws only when something changes draws at each `Redraw`, and after its own changes.
- **Resizing on Windows.** While a person drags a window's border, Windows runs its own loop, and the window's events wait. A window that `std.gpu` (14.24) has drawn into is drawn again at each step: after each `Redraw`, the system waits up to 50 ms for the program's next `frame.show()`, so the picture follows the border instead of stretching. A program that doesn't draw in time is not stopped: the resize goes on.

## 14.22.2 Text input

A text box needs more than a game: what each key means, keys that repeat, which modifiers are held, clicks counted, and the text an input method composes before it is typed. They all come in the one `Event` stream; a program that doesn't read a field leaves it.

```kavach
fn edit(box: TextBox, e: Event) -> TextBox {
    match e {
        Event.Key(_, true, "Backspace", _, _) => box.delete_back(),
        Event.Key(_, true, "z", _, m) if m.ctrl => box.undo(),
        Event.Text(text) => box.insert(text),
        Event.Composing(text, from, to) => box.composing(text, from, to),
        Event.MouseButton(0, true, 2, _) => box.select_word(),
        _ => box,
    }
}
```

- **Names and places.** `key` stays the key's place, for games and shortcuts by place; `name` is what the layout makes of it, for shortcuts by letter (Ctrl+Z is the key named `z`, wherever a layout puts it). Typed text comes as `Text`, not from `name`: a dead key, an input method, or Shift with another key all change what is typed.
- **Repeats.** A key held down gives `Key` events with `repeat` true at the system's repeat rate, with no `Key` up between them, and a `Text` each time when it types.
- **Clicks.** A press of the same button within 500 ms and 4 pixels of the last press makes `clicks` one more; otherwise it is 1. Its release has the same `clicks`.
- **Input methods.** Chinese, Japanese, Korean and many other languages are typed through an input method: a person types keys, picks from a list, and only then is text typed. `win.set_text_input(true)` turns them on for the window, when a text box has the focus; `set_text_input(false)` when none has, so a game's keys go to the game. While they compose, `Composing` events say what to show; `Text` gives what was typed in the end. `win.set_caret(x, y, width, height)` says where the caret is, so the input method shows its list beside it, and is said again when the caret moves.

## Open questions

- macOS shows windows only from a program's first thread. Until the runtime gives that thread to the event loop, `window.open` there fails with `Unavailable`.
- A pen's tilt and its buttons.
- For a maintainer's review (14.22.2): the new fields have defaults, so `Event.Key(key: "KeyW", down: true)` still makes a key; momentum scrolling after a touchpad's gesture comes as more `Wheel` events, with no phase of its own; 500 ms and 4 pixels are the systems' usual double-click limits, not each machine's setting.
- For a maintainer's review (14.22.1): `wake()` gives an event of its own, `Woken`, beside the system's `Redraw`, so a program can tell its own wakes from the system's; a program's own "draw again" (`winit`'s `request_redraw`) is left out, as `wake()` covers it. The 50 ms a resize waits for a frame is three frames at 60 Hz. Drawing into a window is `std.gpu` (14.24).
