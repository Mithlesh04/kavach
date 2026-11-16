//! The mixer and the decoders, with no sound device: nothing is heard.

use crate::mixer::{Controls, SoundData, Voice, mix};
use std::sync::Arc;

fn voice(samples: Vec<f32>, channels: u16, rate: u32, looped: bool) -> (Voice, Arc<Controls>) {
    let controls = Arc::new(Controls::new(1.0, 0.0));
    let sound = Arc::new(SoundData {
        samples,
        channels,
        rate,
    });
    let v = Voice {
        sound,
        at: 0.0,
        looped,
        controls: controls.clone(),
    };
    (v, controls)
}

#[test]
fn a_mono_sound_is_mixed_into_both_sides_and_ends() {
    let (v, c) = voice(vec![0.5, -0.5], 1, 48_000, false);
    let mut voices = vec![v];
    let mut dead = 0;
    let mut out = [9.0f32; 6];
    mix(&mut out, 2, 48_000, &mut voices, &mut |_| dead += 1);
    assert_eq!(out, [0.5, 0.5, -0.5, -0.5, 0.0, 0.0]);
    assert!(voices.is_empty() && dead == 1);
    assert!(c.done.load(std::sync::atomic::Ordering::Acquire));
}

#[test]
fn volume_and_pan_set_each_side() {
    let (v, c) = voice(vec![1.0; 4], 1, 48_000, false);
    c.set_volume(0.5);
    c.set_pan(1.0);
    let mut voices = vec![v];
    let mut out = [0.0f32; 2];
    mix(&mut out, 2, 48_000, &mut voices, &mut |_| {});
    assert_eq!(out, [0.0, 0.5], "all the way right, at half");
    c.set_pan(-0.5);
    mix(&mut out, 2, 48_000, &mut voices, &mut |_| {});
    assert_eq!(out, [0.5, 0.25], "left, the right side lowered");
}

#[test]
fn a_slower_sound_is_blended_between_its_frames() {
    // At half the device's rate, each sound frame spans two device frames.
    let (v, _) = voice(vec![0.0, 1.0], 1, 24_000, false);
    let mut voices = vec![v];
    let mut out = [0.0f32; 4];
    mix(&mut out, 1, 48_000, &mut voices, &mut |_| {});
    assert_eq!(out, [0.0, 0.5, 1.0, 1.0]);
}

#[test]
fn a_looped_sound_starts_again_and_a_stopped_one_ends() {
    let (v, c) = voice(vec![0.25, 0.75], 1, 48_000, true);
    let mut voices = vec![v];
    let mut out = [0.0f32; 5];
    mix(&mut out, 1, 48_000, &mut voices, &mut |_| {});
    assert_eq!(out, [0.25, 0.75, 0.25, 0.75, 0.25]);
    assert_eq!(voices.len(), 1, "still playing");
    c.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    mix(&mut out, 1, 48_000, &mut voices, &mut |_| {});
    assert!(voices.is_empty());
    assert_eq!(out, [0.0; 5]);
}

#[test]
fn mixed_voices_add_up_and_stay_in_range() {
    let (a, _) = voice(vec![0.75], 1, 48_000, true);
    let (b, _) = voice(vec![0.75], 1, 48_000, true);
    let mut voices = vec![a, b];
    let mut out = [0.0f32; 1];
    mix(&mut out, 1, 48_000, &mut voices, &mut |_| {});
    assert_eq!(out, [1.0], "1.5 is held at 1");
}

/// A WAV file of 16-bit samples.
fn wav16(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(&data);
    b
}

#[test]
fn a_wav_file_is_read() {
    let file = wav16(2, 44_100, &[16_384, -16_384, 0, 32_767]);
    let n = crate::decode(&file).expect("read");
    assert_eq!(crate::info(n), (44_100, 2, 2));
    let s = crate::decode::decode(&file).expect("read");
    assert_eq!(s.samples[..3], [0.5, -0.5, 0.0]);
    crate::drop_sound(n);
    assert_eq!(crate::info(n), (0, 0, 0));
}

#[test]
fn what_is_not_a_sound_file_is_refused() {
    for (bytes, why) in [
        (b"hello".to_vec(), "not a WAV or Ogg Vorbis"),
        (wav16(1, 44_100, &[1, 2])[..30].to_vec(), "can't be read"),
        (
            b"OggS but not really a vorbis stream".to_vec(),
            "Ogg Vorbis file can't be read",
        ),
    ] {
        let err = crate::decode(&bytes).expect_err("refused");
        assert!(err.contains(why), "{err}");
    }
}

#[test]
fn a_made_sound_keeps_its_samples_in_range() {
    let n = crate::sound(vec![2.0, -3.0, f32::NAN], 22_050, 2);
    assert_eq!(crate::info(n), (22_050, 2, 1), "the odd sample is dropped");
}

/// A quarter second of a 440 Hz tone at half volume, stereo, 22,050
/// frames a second, encoded by `oggenc` (`tone.ogg`).
#[test]
fn an_ogg_vorbis_file_is_read() {
    let s = crate::decode::decode(include_bytes!("tone.ogg")).expect("read");
    assert_eq!((s.rate, s.channels), (22_050, 2));
    assert_eq!(
        s.frames(),
        5_512,
        "a quarter second, without the last packet's padding"
    );
    let peak = s.samples.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!((0.4..0.6).contains(&peak), "about half volume: {peak}");
}

/// The microphone's ring keeps what fits, drops the rest, and gives
/// each sample once (made-up samples: no microphone is opened).
#[test]
fn a_recording_ring_keeps_what_fits_and_gives_it_once() {
    let ring = crate::input::Ring::new(4);
    ring.write([0.1, 0.2, 0.3].into_iter());
    assert_eq!(ring.take(), vec![0.1, 0.2, 0.3]);
    assert!(ring.take().is_empty(), "taken once");
    ring.write([1.0, 2.0, 3.0, 4.0, 5.0].into_iter());
    assert_eq!(
        ring.take(),
        vec![1.0, 2.0, 3.0, 4.0],
        "the fifth didn't fit"
    );
    ring.write([6.0, 7.0].into_iter());
    assert_eq!(ring.take(), vec![6.0, 7.0], "room again after a take");
}

/// Of four interleaved channels, the first two are kept.
#[test]
fn a_recording_keeps_the_first_two_channels() {
    let data = [1, 2, 3, 4, 5, 6, 7, 8];
    let kept: Vec<i32> = crate::input::first_two(&data, 4).collect();
    assert_eq!(kept, vec![1, 2, 5, 6]);
    let mono: Vec<i32> = crate::input::first_two(&data[..3], 1).collect();
    assert_eq!(mono, vec![1, 2, 3]);
}
