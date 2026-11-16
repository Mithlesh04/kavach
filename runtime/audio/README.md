# runtime/audio/

Sound for `std.audio` (spec 14.27), through `cpal` (decision 0114): sounds and voices by number, mixed on the sound device's real-time thread, and recordings from the microphone. Apart from `native/`, so a program without sound doesn't link `cpal`, nor `libasound` on Linux (decision 0024).

| File | Job |
|---|---|
| `Cargo.toml` | The crate `kavach-runtime-audio`: `cpal` for the device, `lewton` for Ogg Vorbis |
| `src/lib.rs` | The program's side: sounds and voices by number, and the calls the interpreter and native code make: `sound`, `decode`, `info`, `drop_sound`, `play`, `stop`, `set_volume`, `set_pan`, `playing`, and for the microphone `listen`, `take`, `stop_listening`, `drop_recording`, `record` |
| `src/mixer.rs` | The mixer: every voice added into the device's buffer, with volume, pan, rate and looping, never allocating, locking or waiting |
| `src/device.rs` | The default output, opened at the first `play` on a thread of its own, with the bounded channels that take voices to the mixer and back |
| `src/input.rs` | The default microphone, opened by each `listen` on a thread of its own; its callback writes into a ring made before it starts, which `take` empties |
| `src/decode.rs` | WAV files, read here, and Ogg Vorbis files, through `lewton` |
| `src/tests.rs` | The mixer, the decoders and the microphone's ring with no sound device: nothing is heard or recorded |
| `src/tone.ogg` | A quarter second of a 440 Hz tone, an Ogg Vorbis file for the decoding test (made with `sox` and `oggenc`) |

Building Kavach on Linux needs `libasound` for `cpal`: install `libasound2-dev`, or point `PKG_CONFIG` at a `pkg-config` that knows where its files are.
