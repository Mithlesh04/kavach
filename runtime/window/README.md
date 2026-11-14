# runtime/window/

Windows and the keyboard and mouse input that comes to them, for `std.window` (spec 14.22), through `winit` (decision 0104). Apart from `native/`, so a program without windows doesn't link `winit` (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-window` and its one dependency, `winit` |
| `src/lib.rs` | The event loop on a thread of its own, windows by number, each window's inbox of events, and the calls the interpreter and native code make: `open`, `take`, `listen_input`, `size`, `scale`, `set_title`, `set_fullscreen`, `lock_cursor`, `close`, and `with_window`, which runs code on the event loop's thread with a window, as `std.gpu` needs to make its surface |
