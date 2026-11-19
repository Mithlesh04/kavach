// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Sound for `std.audio` (spec 14.27, decision 0114), through `cpal`:
//! sounds and voices by number, as the interpreter and native code call
//! them. The program's side keeps the sounds and each voice's controls;
//! the mixer (`mixer.rs`) plays on the device's real-time thread
//! (`device.rs`). Decoding is `decode.rs`, and the microphone `input.rs`.

mod decode;
mod device;
mod input;
pub mod mixer;
mod stream;
#[cfg(test)]
mod tests;

use mixer::{Controls, SoundData, Voice};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[derive(Default)]
struct Store {
    sounds: HashMap<i64, Arc<SoundData>>,
    voices: HashMap<i64, Arc<Controls>>,
    recordings: HashMap<i64, input::Listening>,
    next: i64,
}

fn store() -> MutexGuard<'static, Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Frees what the mixer finished with, here, off its thread; and forgets
/// voices that are done.
fn tidy(s: &mut Store) {
    if let Ok(out) = device::output()
        && let Ok(freed) = out.freed.lock()
    {
        while freed.try_recv().is_ok() {}
    }
    s.voices
        .retain(|_, c| !c.done.load(std::sync::atomic::Ordering::Acquire));
}

fn keep(sound: SoundData) -> i64 {
    let mut s = store();
    s.next += 1;
    let n = s.next;
    s.sounds.insert(n, Arc::new(sound));
    n
}

/// A sound of `samples` (interleaved, `channels` of 1 or 2, each from -1
/// to 1) at `rate` frames a second: its number.
pub fn sound(samples: Vec<f32>, rate: u32, channels: u16) -> i64 {
    let channels = channels.clamp(1, 2);
    let mut samples = samples;
    samples.truncate(samples.len() - samples.len() % usize::from(channels));
    for x in &mut samples {
        *x = if x.is_nan() { 0.0 } else { x.clamp(-1.0, 1.0) };
    }
    keep(SoundData {
        samples,
        channels,
        rate: rate.clamp(1_000, 384_000),
    })
}

/// The sound in a WAV or Ogg Vorbis file: its number, or why not.
pub fn decode(bytes: &[u8]) -> Result<i64, String> {
    decode::decode(bytes).map(keep)
}

/// Sound `n`'s rate, channels and frames; zeros once it is gone.
pub fn info(n: i64) -> (i64, i64, i64) {
    store().sounds.get(&n).map_or((0, 0, 0), |s| {
        (i64::from(s.rate), i64::from(s.channels), s.frames() as i64)
    })
}

/// Lets sound `n` go; voices playing it play on.
pub fn drop_sound(n: i64) {
    store().sounds.remove(&n);
}

/// Plays sound `n` at `volume` (0 is silent, 1 as it is) and `pan` (-1
/// left to 1 right), from its start, over and over when `looped`: the
/// voice's number, or why nothing can play.
pub fn play(n: i64, volume: f64, pan: f64, looped: bool) -> Result<i64, String> {
    let out = device::output()?;
    let mut s = store();
    tidy(&mut s);
    let sound = s.sounds.get(&n).cloned().ok_or("this sound is gone")?;
    let controls = Arc::new(Controls::new(1.0, 0.0));
    controls.set_volume(volume as f32);
    controls.set_pan(pan as f32);
    let voice = Voice {
        sound,
        at: 0.0,
        looped,
        controls: controls.clone(),
        streaming: None,
    };
    out.add
        .try_send(voice)
        .map_err(|_| format!("{} voices are playing already", device::MOST_VOICES))?;
    s.next += 1;
    let v = s.next;
    s.voices.insert(v, controls);
    Ok(v)
}

/// Plays the WAV or Ogg Vorbis file at `path` a piece at a time, as
/// `play` plays a sound (spec 14.27): the voice's number; or why not, a
/// file that can't be read first (`Err(Ok(..))`), then no device
/// (`Err(Err(..))`).
pub fn stream(
    path: &str,
    volume: f64,
    pan: f64,
    looped: bool,
) -> Result<i64, Result<String, String>> {
    let controls = Arc::new(Controls::new(1.0, 0.0));
    controls.set_volume(volume as f32);
    controls.set_pan(pan as f32);
    let source = stream::start(path, looped, controls.clone()).map_err(Ok)?;
    let out = match device::output() {
        Ok(out) => out,
        Err(why) => {
            controls
                .stop
                .store(true, std::sync::atomic::Ordering::Relaxed);
            return Err(Err(why));
        }
    };
    let empty = Arc::new(SoundData {
        samples: Vec::new(),
        channels: 2,
        rate: source.rate,
    });
    let voice = Voice {
        sound: empty,
        // Two frames are taken before the first is heard.
        at: 2.0,
        looped,
        controls: controls.clone(),
        streaming: Some(mixer::Streaming {
            source,
            frames: [(0.0, 0.0); 2],
        }),
    };
    if out.add.try_send(voice).is_err() {
        controls
            .stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        return Err(Err(format!(
            "{} voices are playing already",
            device::MOST_VOICES
        )));
    }
    let mut s = store();
    tidy(&mut s);
    s.next += 1;
    let v = s.next;
    s.voices.insert(v, controls);
    Ok(v)
}

/// Voice `v`'s controls, while it plays.
fn controls(v: i64) -> Option<Arc<Controls>> {
    store().voices.get(&v).cloned()
}

/// Stops voice `v`; nothing for one that ended.
pub fn stop(v: i64) {
    if let Some(c) = controls(v) {
        c.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Voice `v`'s volume from now on.
pub fn set_volume(v: i64, volume: f64) {
    if let Some(c) = controls(v) {
        c.set_volume(volume as f32);
    }
}

/// Voice `v`'s pan from now on.
pub fn set_pan(v: i64, pan: f64) {
    if let Some(c) = controls(v) {
        c.set_pan(pan as f32);
    }
}

/// Whether voice `v` still plays.
pub fn playing(v: i64) -> bool {
    controls(v).is_some_and(|c| !c.done.load(std::sync::atomic::Ordering::Acquire))
}

/// Starts recording from the default microphone: the recording's number,
/// or why it can't.
pub fn listen() -> Result<i64, String> {
    let recording = input::listen()?;
    let mut s = store();
    s.next += 1;
    let n = s.next;
    s.recordings.insert(n, recording);
    Ok(n)
}

/// What recording `n` heard since the last take, as a new sound's number;
/// an empty sound once the recording is gone.
pub fn take(n: i64) -> i64 {
    let (samples, rate, channels) = match store().recordings.get(&n) {
        Some(r) => (r.ring.take(), r.rate, r.channels),
        None => (Vec::new(), 48_000, 1),
    };
    keep(SoundData {
        samples,
        channels,
        rate,
    })
}

/// Stops recording `n`; what it heard stays to be taken.
pub fn stop_listening(n: i64) {
    if let Some(r) = store().recordings.get(&n) {
        r.stop();
    }
}

/// Lets recording `n` go, stopping it.
pub fn drop_recording(n: i64) {
    if let Some(r) = store().recordings.remove(&n) {
        r.stop();
    }
}

/// Records `seconds` (at most an hour) and gives the sound's number,
/// taking every second so the ring never fills.
pub fn record(seconds: f64) -> Result<i64, String> {
    let seconds = seconds.clamp(0.0, 3600.0);
    let recording = input::listen()?;
    let (rate, channels) = (recording.rate, recording.channels);
    let want = (seconds * f64::from(rate)) as usize * usize::from(channels);
    let start = std::time::Instant::now();
    let mut samples = Vec::with_capacity(want);
    while samples.len() < want {
        let left = seconds - start.elapsed().as_secs_f64();
        std::thread::sleep(std::time::Duration::from_secs_f64(left.clamp(0.001, 1.0)));
        samples.extend(recording.ring.take());
        if left <= 0.0 && samples.len() < want {
            // The device gave less than asked for: what it gave.
            break;
        }
    }
    recording.stop();
    samples.truncate(want);
    Ok(keep(SoundData {
        samples,
        channels,
        rate,
    }))
}

/// Starts a voice the program fills itself (`audio.voice`, spec 14.27),
/// at `volume` and `pan`: its number, or why nothing can play.
pub fn voice(volume: f64, pan: f64) -> Result<i64, String> {
    let out = device::output()?;
    let controls = Arc::new(Controls::new(1.0, 0.0));
    controls.set_volume(volume as f32);
    controls.set_pan(pan as f32);
    let source = stream::fed();
    let voice = Voice {
        sound: Arc::new(SoundData {
            samples: Vec::new(),
            channels: 2,
            rate: source.rate,
        }),
        at: 2.0,
        looped: false,
        controls: controls.clone(),
        streaming: Some(mixer::Streaming {
            source: source.clone(),
            frames: [(0.0, 0.0); 2],
        }),
    };
    out.add
        .try_send(voice)
        .map_err(|_| format!("{} voices are playing already", device::MOST_VOICES))?;
    let mut s = store();
    tidy(&mut s);
    s.next += 1;
    let v = s.next;
    s.voices.insert(v, controls);
    fed_voices().insert(v, source);
    Ok(v)
}

fn fed_voices() -> MutexGuard<'static, HashMap<i64, Arc<stream::Stream>>> {
    static FED: OnceLock<Mutex<HashMap<i64, Arc<stream::Stream>>>> = OnceLock::new();
    FED.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// How many frames voice `v` takes now to be about 50 ms ahead; none for
/// a voice that isn't one the program fills, or has stopped.
pub fn voice_room(v: i64) -> i64 {
    if !playing(v) {
        fed_voices().remove(&v);
        return 0;
    }
    fed_voices().get(&v).map_or(0, |s| stream::room(s) as i64)
}

/// Queues `samples` (left and right in turn) on voice `v`; what doesn't
/// fit is dropped.
pub fn voice_feed(v: i64, samples: &[f32]) {
    if let Some(s) = fed_voices().get(&v) {
        let whole = samples.len() & !1;
        s.ring.write(samples[..whole].iter().copied());
    }
}
