# runtime/input/

Gamepads for `std.input` (spec 14.26), through SDL3 (decision 0124): each connected pad's state with its gyroscope, its events given to the windows that listen to input (`runtime/window`), and rumble. SDL3 is built from its source and linked in, so a Kavach program's author installs nothing. Apart from `native/`, so a program that reads no gamepads doesn't link SDL (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-input`: `sdl3`, built from source with its sensors, and `kavach-runtime-window`, whose windows get the pads' events |
| `build.rs` | Stops the build at once when CMake is missing, with the command that installs it on this system |
| `src/lib.rs` | The standard layout's names, the thread that watches pads, their state from each event and each gyroscope, and the calls the interpreter and native code make: `start`, `pads`, `pad_numbers`, `names`, `rumble_pad` |
| `src/tests.rs` | The names and the state kept from events, with events made up in the test: no pad needs to be connected |

Building Kavach needs CMake, which builds SDL3: `winget install Kitware.CMake`, `brew install cmake`, or your Linux's `cmake` package (`sudo apt install cmake`).
