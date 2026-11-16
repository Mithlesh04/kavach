# 14.27 std.audio

Sound: effects that start the moment something happens, and music ([decision 0114](../../decisions/0114-sound-through-cpal.md)). A program loads or makes a `Sound` and plays it. The runtime mixes every playing sound on the sound device's own thread, so a slow frame or a pause in the program never makes the sound click.

Playing sound needs `Audio` (8.1), and recording what the microphone hears needs `Microphone`. Reading or making a sound needs nothing: a sound is data until it plays.

```kavach
use std.audio
use std.fs

fn main() ! Error uses Audio, Fs.read {
    const boom = try audio.decode(try fs.read_bytes("boom.ogg"))
    const music = try audio.decode(try fs.read_bytes("theme.wav"))
    const theme = try audio.play(music, volume: 0.6, looped: true)
    try audio.play(boom, pan: -0.5)
    theme.set_volume(0.3)
}
```

| Function or method | Gives |
|---|---|
| `audio.decode(bytes) -> Sound ! AudioError` | The sound in the bytes of a WAV file (whole-number samples of 8, 16, 24 or 32 bits, or 32-bit floats) or an Ogg Vorbis file, told apart by their first bytes |
| `audio.sound(samples, rate, channels) -> Sound` | A sound of `samples`, a `[Float32]` each from -1 to 1, at `rate` frames a second (1,000 to 384,000), with 1 or 2 `channels`; with two, left and right take turns |
| `sound.rate`, `sound.channels`, `sound.frames: Int` | Its frames a second, its channels, and how many frames (one sample of each channel) it has |
| `sound.seconds() -> Float` | How long it lasts |
| `audio.play(sound, volume: 1.0, pan: 0.0, looped: false) -> Voice ! AudioError` | Starts playing `sound` and gives at once: `volume` 0 is silent, 1 the sound as it is (up to 4); `pan` -1 is left, 1 right; `looped` plays it over and over. Needs `Audio` |
| `audio.stream(path, volume: 1.0, pan: 0.0, looped: false) -> Voice ! AudioError` | Plays the WAV or Ogg Vorbis file at `path` as `play` plays a sound, reading it a piece at a time: for long music, which `decode` would hold whole in memory. A file that can't be read or isn't one of those is `BadSound`. Needs `Audio` and `Fs.read` |
| `audio.voice(volume: 1.0, pan: 0.0) -> Voice ! AudioError` | A voice the program fills with sound it makes, 48,000 frames a second: for synthesizers and sound made as a game runs. Needs `Audio` |
| `voice.fill(state, next) -> S` | For a voice from `audio.voice`: makes as many frames as it has room for, about 50 ms ahead, each by `next: fn(S) -> (S, Float32, Float32)` (the state after the frame, then its left and right), from `state`; gives the state after them. Needs `Audio` |
| `voice.stop()` | Stops it at once. Needs `Audio`, as do the three below |
| `voice.set_volume(volume)`, `voice.set_pan(pan)` | Changes it while it plays |
| `voice.is_playing() -> Bool` | Whether it still plays: `false` once it ended or was stopped |

**Voices you fill.** `audio.voice()` gives a voice whose sound the program makes itself, a frame at a time: a synthesizer, an engine's hum that follows its speed. Each frame of the program's loop, `voice.fill(state, next)` calls `next` for every frame the voice has room for, about 50 ms ahead, and queues them. The state goes from one call to the next, so a function can keep a phase, a filter or an envelope. If the program stops calling `fill`, the voice goes quiet when its queue runs out, and starts again when called.

```kavach
use std.audio
use std.math
use std.time

fn tone(phase: Float) -> (Float, Float32, Float32) {
    const s = Float32(math.sin(phase) * 0.2)
    (phase + 2.0 * math.pi * 440.0 / 48_000.0, s, s)      // A 440 Hz sine, both sides
}

fn play_tone(seconds: Int) ! AudioError uses Audio, Clock {
    const synth = try audio.voice()
    let phase = 0.0
    for _ in 0..seconds * 60 {
        phase = synth.fill(phase, tone)
        time.sleep(16.ms)                                 // a game's frame
    }
}
```

**Recording.** `audio.listen()` starts recording and gives a `Recording` at once; each `take()` gives what was heard since the one before, so a loop can show a level meter or send the voice on. `audio.record(seconds)` records a clip and waits for it.

```kavach
use std.audio
use std.time

fn main() ! Error uses Microphone, Audio {
    const clip = try audio.record(3.0)          // three seconds
    const _ = try audio.play(clip)
    const rec = try audio.listen()
    for _ in 0..10 {
        time.sleep(100.ms)
        const heard = rec.take()                // about 0.1 seconds
        print(heard.frames)
    }
    rec.stop()
}
```

| Function or method | Gives |
|---|---|
| `audio.listen() -> Recording ! AudioError` | Starts recording what the system's default microphone hears, at its own rate and with its first two channels, and gives at once. Needs `Microphone` |
| `rec.take() -> Sound` | What it heard since the last take, or since it started, as a sound; an empty one when nothing new. Between takes it keeps up to 60 seconds; past that, what it hears is dropped until the next take. Needs `Microphone`, as `stop` does |
| `rec.stop()` | Stops recording; a take after it gives what was left. Letting a `Recording` go stops it too |
| `audio.record(seconds) -> Sound ! AudioError` | Records for `seconds` (more than 0, at most 3,600), waiting meanwhile, and gives the sound. Needs `Microphone` |

- `AudioError` is `Unavailable(reason: String)`, when nothing can play or record (the machine has no sound device or microphone, as a server, or 256 voices play already), or `BadSound(reason: String)`, when the bytes aren't a sound file Kavach reads.
- A file of more than two channels keeps its first two. A sound at another rate than the device's is played at the device's rate, blended between its frames.
- Voices add up. Where they add past 1, the sound is held at 1.
- Letting a `Voice` go doesn't stop it, so `audio.play(boom)` alone plays the whole sound. Letting a `Sound` go while it plays plays it to its end.
- Generated tests (11.5) skip functions that use `Audio` or `Microphone`, so a test run plays and records nothing.

## Open questions

- Choosing the microphone, and its rate, instead of the system's default.
- A voice whose samples a Kavach function makes on the sound device's own thread, with no time ahead: it needs a calling rule that never allocates, which the compiler doesn't have yet.
- Choosing the output device.
- Opus, which waits for a decoder that needs no C library.
