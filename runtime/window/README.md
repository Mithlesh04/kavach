# runtime/window/

Windows and the keyboard and mouse input that comes to them, for `std.window` (spec 14.22), through `winit` (decision 0104). Apart from `native/`, so a program without windows doesn't link `winit` (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-window` and its one dependency, `winit` |
| `src/lib.rs` | The event loop on a thread of its own, windows by number, and the calls the interpreter and native code make: `open`, `set_title`, `set_fullscreen`, `lock_cursor`, `close`, and `with_window`, which runs code on the event loop's thread with a window, as `std.gpu` needs to make its surface. After a `Redraw`, it waits up to 50 ms for a frame of a window drawn into, so a resize is drawn as it goes (spec 14.22.1) |
| `src/inbox.rs` | Each window's inbox of events: `take`, `ready`, `on_event` (a task waiting while others run leaves its waker), `wake`, `listen_input`, `deliver_input` (a gamepad's event, from `runtime/input`, to every window that listens to input), `size`, `scale`, and `frame_shown`, which `std.gpu` calls at each frame shown |
| `src/events.rs` | `winit`'s events turned into `std.window`'s, numbered as its `Event` variants are |
| `src/errors.rs` | Windowing errors as a program sees them: the reason, without where in winit's source it failed |
| `src/tests.rs` | Events translated (input only after listening, touches with their phase, id and pressure), wakes and redraws coming once, waiting tasks woken, the wait for a frame, and a window opened or refused |
