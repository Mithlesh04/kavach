# 14.27 std.audio

Sound: effects that start the moment something happens, and music ([decision 0114](../../decisions/0114-sound-through-cpal.md)). A program loads or makes a `Sound` and plays it. The runtime mixes every playing sound on the sound device's own thread, so a slow frame or a pause in the program never makes the sound click.

Playing sound needs `Audio` (8.1). Reading or making a sound needs nothing: a sound is data until it plays.

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
| `voice.stop()` | Stops it at once. Needs `Audio`, as do the three below |
| `voice.set_volume(volume)`, `voice.set_pan(pan)` | Changes it while it plays |
| `voice.is_playing() -> Bool` | Whether it still plays: `false` once it ended or was stopped |

- `AudioError` is `Unavailable(reason: String)`, when nothing can play (the machine has no sound device, as a server, or 256 voices play already), or `BadSound(reason: String)`, when the bytes aren't a sound file Kavach reads.
- A file of more than two channels keeps its first two. A sound at another rate than the device's is played at the device's rate, blended between its frames.
- Voices add up. Where they add past 1, the sound is held at 1.
- Letting a `Voice` go doesn't stop it, so `audio.play(boom)` alone plays the whole sound. Letting a `Sound` go while it plays plays it to its end.
- Generated tests (11.5) skip functions that use `Audio`, so a test run plays nothing.

## Open questions

- Recording, behind `Microphone` (8.1): `audio.record(…)` and a stream of what the microphone hears. Next, in this module.
- A voice whose samples a Kavach function makes as it plays, for synthesizers, run on the sound device's thread. The compiler must check that the function never allocates memory, locks or waits ([decision 0114](../../decisions/0114-sound-through-cpal.md)).
- Streaming long music from a file instead of decoding it whole first, and choosing the output device.
- Opus, which waits for a decoder that needs no C library.
