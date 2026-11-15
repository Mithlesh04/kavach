# 0114. Sound through `cpal`: a runtime mixer, `Audio` and `Microphone`

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

Games, media apps and tools need sound: effects that start the moment something happens, music that streams, and sometimes the microphone. Kavach has no sound at all. This is language request 9 (4) in the to-do.

Sound has hard timing. The system asks for the next few milliseconds of samples on a real-time thread, and if they come late the person hears a click. So code on that thread must never allocate memory, take a lock another thread holds, or wait. Each system has its own way in: WASAPI on Windows, ALSA (with PipeWire or PulseAudio behind it) on Linux, CoreAudio on macOS. A maintainer chose the library, the shape of the API, the capabilities and the file formats (2026-11-15).

## Decision

- **`cpal`** (Apache-2.0) reaches the speakers and the microphone on Windows, Linux and macOS, and runs the real-time thread.
- **A runtime mixer first.** A program loads or makes a `Sound` and calls `audio.play(sound, volume:, pan:, looped:)`, which gives a `Voice` it can change or stop. The runtime mixes every voice on the real-time thread. No Kavach code runs there, so nothing a program does can make it late. The program's changes reach the mixer through a queue that never blocks.
- **A Kavach callback voice later**, for synthesizers: a function that fills each buffer of samples, run on the real-time thread. It needs a compiler check that the function never allocates, locks or waits, so it is a stage of its own, with its own spec section.
- **Two capabilities** (spec 8.1): `Audio` plays sound, and `Microphone` records it. A game that plays sound can't listen to the room, as a function that draws can't read the keyboard (decision 0104).
- **WAV and Ogg Vorbis in std.** WAV is read by Kavach's runtime itself; Ogg Vorbis through the pure-Rust `lewton` crate (MIT or Apache-2.0). Opus waits for a decoder that needs no C library. Packages add other formats by giving `audio.sound` the samples.
- **On Linux, `cpal` links `libasound`.** Only a program that plays or records sound links it (decision 0024), as `libasound.so.2`, which every desktop Linux has. Building Kavach itself on Linux needs `libasound2-dev` (or its files) for the `pkg-config` check of `alsa-sys`.

## Consequences

- Programs play sound with a few calls, and a slow frame or a pause in the program never makes the sound click.
- A machine with no sound device (a server, a container) can't play: `audio.play` fails with `AudioError.Unavailable`, which a program can ignore.
- Tests can't listen. They check decoding, mixing into a buffer, and the voices' lifecycle without a device, and play only silence on a real one.
- `cpal` is Apache-2.0 only; Kavach (Apache-2.0 or MIT) may depend on it, and a program built with it carries its notice.

## Alternatives considered

- **miniaudio through C:** one C file with every backend and decoders for WAV, MP3 and FLAC. Rejected: a C library to build with every program that plays sound.
- **Kavach's own code for each system:** WASAPI and ALSA through `windows-sys` and `libc`. No dependency, but much more code to keep up, and macOS later still. Rejected for now.
- **Only a callback:** full control, but every program would need the real-time check from the start, and most programs only want to play sounds. The mixer comes first.
- **One capability, or none to play:** a single `Audio` would let any code that plays sound also record; none to play would hide noise-making code from `kavach audit`. A maintainer chose `Audio` and `Microphone`.
