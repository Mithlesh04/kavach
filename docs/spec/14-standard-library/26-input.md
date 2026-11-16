# 14.26 std.input

Gamepads, in one standard layout, whatever pad is plugged in, with their gyroscopes ([decision 0124](../../decisions/0124-gamepads-through-sdl3.md)). The keyboard and the mouse come to windows (14.22). Reading gamepads needs `Input` (8.1), as reading the keyboard does.

A program sees gamepads two ways:

- as **events** in a window's stream, like keys: `PadConnected`, `PadGone`, `PadButton` and `PadAxis` (14.22) come to every window that listens to input;
- as **state**: `input.pads()` gives each connected pad now, for a game loop that reads it once a frame.

```kavach
use std.input

fn steer(ship: Ship) -> Ship uses Input {
    for pad in input.pads() {
        if pad.is_down("South") {
            pad.rumble(0.5, 0.2, 100)
            return ship.jump()
        }
        return ship.turn(pad.left.x)
    }
    ship
}
```

| Function or method | Gives |
|---|---|
| `input.pads() -> [Pad]` | The gamepads connected now, by their numbers. The first call starts watching them |
| `input.buttons() -> [String]` | The buttons of the standard layout, named by their place (below) |
| `pad.is_down(button) -> Bool` | Whether the button is held |
| `pad.rumble(strong, weak, millis)` | Makes the pad vibrate for `millis` milliseconds (0 to 60,000): its strong (low) motor at `strong` and its weak (high) one at `weak`, each from 0 to 1. Nothing happens for a pad that is gone or can't rumble |

A `Pad` is a record:

| Field | Holds |
|---|---|
| `id: Int` | The pad's number, as its events give it |
| `name: String` | Its name, as the system gives it |
| `buttons: [String]` | The buttons held now |
| `left`, `right: (x: Float, y: Float)` | Where each stick is: `x` from -1 (left) to 1 (right), `y` from -1 (down) to 1 (up) |
| `left_trigger`, `right_trigger: Float` | How far each trigger is pressed, from 0 to 1 |
| `gyro: (x: Float, y: Float, z: Float)?` | How fast the pad turns, in radians a second, about its x axis (pitch, its far edge up), y (yaw, turning left) and z (roll, its right side down); `none` for a pad with no gyroscope, and by default in a `Pad` made by hand |

The buttons are named by their place, not by the letter printed on them, as keys are named by their place (14.22): `South` is A on an Xbox pad and Cross on a PlayStation pad. They are `South`, `East`, `North`, `West`, `LeftBumper`, `RightBumper`, `LeftTrigger`, `RightTrigger`, `Select`, `Start`, `Mode`, `LeftStick` and `RightStick` (the sticks pressed in), `DPadUp`, `DPadDown`, `DPadLeft` and `DPadRight`.

- A trigger is a button and an axis: `LeftTrigger` is held once it is pressed far enough, and its axis says how far.
- A small movement around a stick's middle counts as no movement, so a pad left alone reads 0.
- The gyroscope is state only: it comes as no events, which would arrive a thousand times a second. A game reads it once a frame, multiplied by the frame's time, to turn its camera.
- A machine where gamepads can't be read (no udev on Linux, as in a container) has no pads: `input.pads()` is empty and no pad events come. That is not an error.
- Generated tests (11.5) skip functions that use `Input`, as they skip those that use `Window`.

## Open questions

- Accelerometers, the touchpads of some pads, and each pad's battery.
- Mapping a pad the standard layout doesn't know, from the program.
