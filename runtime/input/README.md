# runtime/input/

Gamepads for `std.input` (spec 14.26), through `gilrs` (decision 0113): each connected pad's state, its events given to the windows that listen to input (`runtime/window`), and rumble. Apart from `native/`, so a program that reads no gamepads doesn't link `gilrs`, nor `libudev` on Linux (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-input`: `gilrs`, and `kavach-runtime-window`, whose windows get the pads' events |
| `src/lib.rs` | The standard layout's names, the thread that watches pads, their state from each event, and the calls the interpreter and native code make: `start`, `pads`, `pad_numbers`, `names`, `rumble_pad` |
| `src/tests.rs` | The names and the state kept from events, with events made up in the test: no pad needs to be connected |

Building Kavach on Linux needs `libudev` for `gilrs`: install `libudev-dev`, or point `PKG_CONFIG` at a `pkg-config` that knows where its files are.
