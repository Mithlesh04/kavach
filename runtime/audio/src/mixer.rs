// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The mixer (decision 0114): every playing voice added into the
//! device's buffer, on the real-time thread. It never allocates, locks or
//! waits: a voice's volume, pan and stop are atomics the program sets,
//! and a finished voice goes back to the program's side to be freed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// A sound's samples: interleaved, one or two channels, at `rate` frames
/// a second, each from -1 to 1.
#[derive(Debug)]
pub struct SoundData {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub rate: u32,
}

impl SoundData {
    /// How many frames (one sample of each channel) it has.
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    /// Frame `k`, as left and right.
    fn frame(&self, k: usize) -> (f32, f32) {
        if self.channels == 2 {
            (self.samples[2 * k], self.samples[2 * k + 1])
        } else {
            (self.samples[k], self.samples[k])
        }
    }
}

/// What the program changes in a playing voice, and whether it is done.
#[derive(Debug)]
pub struct Controls {
    volume: AtomicU32,
    pan: AtomicU32,
    pub(crate) stop: AtomicBool,
    pub(crate) done: AtomicBool,
}

impl Controls {
    pub(crate) fn new(volume: f32, pan: f32) -> Self {
        Controls {
            volume: AtomicU32::new(volume.to_bits()),
            pan: AtomicU32::new(pan.to_bits()),
            stop: AtomicBool::new(false),
            done: AtomicBool::new(false),
        }
    }

    pub(crate) fn set_volume(&self, v: f32) {
        self.volume
            .store(v.clamp(0.0, 4.0).to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn set_pan(&self, p: f32) {
        self.pan
            .store(p.clamp(-1.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    /// The left and right gains: the volume, the far side lowered by pan.
    fn gains(&self) -> (f32, f32) {
        let v = f32::from_bits(self.volume.load(Ordering::Relaxed));
        let p = f32::from_bits(self.pan.load(Ordering::Relaxed));
        (v * (1.0 - p).min(1.0), v * (1.0 + p).min(1.0))
    }
}

/// A sound playing: where it is, in frames of the sound.
#[derive(Debug)]
pub struct Voice {
    pub(crate) sound: Arc<SoundData>,
    pub(crate) at: f64,
    pub(crate) looped: bool,
    pub(crate) controls: Arc<Controls>,
    /// Played from its file a piece at a time instead (`stream.rs`): the
    /// frames before and after where it is, from `at`'s 0 to 1.
    pub(crate) streaming: Option<Streaming>,
}

/// A voice's place in a stream: its source and the two frames around it.
#[derive(Debug)]
pub struct Streaming {
    pub(crate) source: Arc<crate::stream::Stream>,
    pub(crate) frames: [(f32, f32); 2],
}

impl std::fmt::Debug for crate::stream::Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream({} frames a second)", self.rate)
    }
}

/// Mixes `voices` into `out`, `channels` samples a frame at `rate` frames
/// a second, from silence. A voice that ends or is stopped is marked done
/// and given to `dead`.
pub fn mix(
    out: &mut [f32],
    channels: usize,
    rate: u32,
    voices: &mut Vec<Voice>,
    dead: &mut impl FnMut(Voice),
) {
    out.fill(0.0);
    let channels = channels.max(1);
    let mut i = 0;
    while i < voices.len() {
        let finished = play_into(out, channels, rate, &mut voices[i]);
        if finished {
            let v = voices.swap_remove(i);
            v.controls.done.store(true, Ordering::Release);
            dead(v);
        } else {
            i += 1;
        }
    }
    for s in out.iter_mut() {
        *s = s.clamp(-1.0, 1.0);
    }
}

/// Adds voice `v` into `out`; whether it finished.
fn play_into(out: &mut [f32], channels: usize, rate: u32, v: &mut Voice) -> bool {
    if v.controls.stop.load(Ordering::Relaxed) {
        return true;
    }
    if v.streaming.is_some() {
        return stream_into(out, channels, rate, v);
    }
    let frames = v.sound.frames();
    if frames == 0 {
        return true;
    }
    let step = f64::from(v.sound.rate) / f64::from(rate.max(1));
    let (gl, gr) = v.controls.gains();
    for frame in out.chunks_mut(channels) {
        if v.at >= frames as f64 {
            if !v.looped {
                return true;
            }
            v.at -= frames as f64;
        }
        // Between two frames of the sound, as the device's rate falls.
        let k = v.at as usize;
        let next = if k + 1 < frames {
            k + 1
        } else if v.looped {
            0
        } else {
            k
        };
        let t = (v.at - k as f64) as f32;
        let (a, b) = (v.sound.frame(k), v.sound.frame(next));
        let (l, r) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        if channels == 1 {
            frame[0] += (l * gl + r * gr) / 2.0;
        } else {
            frame[0] += l * gl;
            frame[1] += r * gr;
        }
        v.at += step;
    }
    false
}

/// Adds streamed voice `v` into `out`, frame by frame from its ring;
/// whether it finished. An empty ring that hasn't ended gives silence.
fn stream_into(out: &mut [f32], channels: usize, rate: u32, v: &mut Voice) -> bool {
    let (gl, gr) = v.controls.gains();
    let Some(st) = v.streaming.as_mut() else {
        return true;
    };
    let step = f64::from(st.source.rate) / f64::from(rate.max(1));
    for frame in out.chunks_mut(channels) {
        while v.at >= 1.0 {
            let ring = &st.source.ring;
            let next = match (ring.pop(), ring.pop()) {
                (Some(l), Some(r)) => (l, r),
                _ if st.source.ended.load(Ordering::Acquire) => return true,
                _ => (0.0, 0.0),
            };
            st.frames = [st.frames[1], next];
            v.at -= 1.0;
        }
        let t = v.at as f32;
        let [a, b] = st.frames;
        let (l, r) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        if channels == 1 {
            frame[0] += (l * gl + r * gr) / 2.0;
        } else {
            frame[0] += l * gl;
            frame[1] += r * gr;
        }
        v.at += step;
    }
    false
}
